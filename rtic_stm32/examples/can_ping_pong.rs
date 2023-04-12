#![no_main]
#![no_std]
#![deny(warnings)]

use stm32f446_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [USART1, USART2])]
mod app {
    use bxcan::filter::Mask32;
    use bxcan::{Fifo, Frame, StandardId};
    use core::sync::atomic::{AtomicUsize, Ordering};
    use defmt::*;
    use dwt_systick_monotonic::{DwtSystick, ExtU32};
    use stm32f4xx_hal::{
        can::Can,
        gpio::{
            gpioa::{PA11, PA12, PA5},
            Alternate, Output, PushPull,
        },
        pac::CAN1,
        prelude::*,
    };

    // Needed for scheduling monotonic tasks
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<45_000_000>; // 180 MHz

    // Holds the shared resources (used by multiple tasks)
    // Needed even if we don't use it
    #[shared]
    struct Shared {
        can1: bxcan::Can<Can<CAN1, (PA12<Alternate<9>>, PA11<Alternate<9>>)>>,
    }

    // Holds the local resources (used by a single task)
    // Needed even if we don't use it
    #[local]
    struct Local {
        led: PA5<Output<PushPull>>,
        test_frame: [u8; 8],
    }

    // Atomic counter
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    // The init function is called in the beginning of the program
    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        info!("init");

        // Cortex-M peripherals
        let mut _core: cortex_m::Peripherals = ctx.core;

        // Device specific peripherals
        let mut _device: stm32f4xx_hal::pac::Peripherals = ctx.device;

        // Set up the system clock.
        let rcc = _device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(45.MHz()).freeze(); // Important: 45 MHz is the max for CAN since it has to match the APB1 clock

        debug!("AHB1 clock: {} Hz", clocks.hclk().to_Hz());
        debug!("APB1 clock: {} Hz", clocks.pclk1().to_Hz());

        // Set up the LED. On the Nucleo-F446RE it's connected to pin PA5.
        let gpioa = _device.GPIOA.split();
        let led = gpioa.pa5.into_push_pull_output();

        // Initialize variables for can_send
        let mut test_frame: [u8; 8] = [0; 8];
        test_frame[1] = 1;
        test_frame[2] = 2;
        test_frame[3] = 3;
        test_frame[4] = 4;
        test_frame[5] = 5;
        test_frame[6] = 6;
        test_frame[7] = 7;

        // Set up CAN device 1
        let mut can1 = {
            // CAN pins alternate function 9 as per datasheet
            // https://www.st.com/resource/en/datasheet/stm32f446mc.pdf page 57
            let rx = gpioa.pa11.into_alternate::<9>();
            let tx = gpioa.pa12.into_alternate::<9>();

            // let can = Can::new(dp.CAN1, (tx, rx));
            // or
            let can = _device.CAN1.can((tx, rx));

            info!("CAN1, waiting for 11 recessive bits...");
            bxcan::Can::builder(can)
                // APB1 (PCLK1): 45MHz, Bit rate: 1MBit/s, Sample Point 87.5%
                // Value was calculated with http://www.bittiming.can-wiki.info/
                .set_bit_timing(0x001b0002)
                .set_automatic_retransmit(true)
                // .set_silent(true)
                .enable()
        };

        info!("CAN1, waiting for 11 recessive bits... (done)");

        can1.enable_interrupts({
            use bxcan::Interrupts as If;
            If::FIFO0_MESSAGE_PENDING | If::FIFO0_FULL | If::FIFO0_OVERRUN
        });

        // Configure filters so that can frames can be received.
        can1.modify_filters()
            .enable_bank(0, Fifo::Fifo0, Mask32::accept_all());

        // enable tracing and the cycle counter for the monotonic timer
        _core.DCB.enable_trace();
        _core.DWT.enable_cycle_counter();

        // Set up the monotonic timer
        let mono = DwtSystick::new(&mut _core.DCB, _core.DWT, _core.SYST, clocks.hclk().to_Hz());

        info!("Init done!");
        blink::spawn_after(1.secs()).ok();
        can_send::spawn_after(1.secs()).ok();
        (
            Shared { can1 },
            Local { led, test_frame },
            init::Monotonics(mono),
        )
    }

    // The idle function is called when there is nothing else to do
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            continue;
        }
    }

    // The task functions are called by the scheduler
    #[task(local = [led])]
    fn blink(ctx: blink::Context) {
        ctx.local.led.toggle();
        debug!("Blink!");
        blink::spawn_after(1.secs()).ok();
    }

    // send a meesage via CAN
    #[task(shared = [can1], local = [test_frame], priority=2)]
    fn can_send(mut ctx: can_send::Context) {
        let test_frame = ctx.local.test_frame;
        let id: u16 = 0x500;

        test_frame[0] = COUNTER.fetch_add(1, Ordering::SeqCst) as u8;
        let frame = Frame::new_data(StandardId::new(id).unwrap(), *test_frame);

        info!("Sending frame with first byte: {}", test_frame[0]);

        ctx.shared.can1.lock(|can1| can1.transmit(&frame).unwrap());
    }

    // receive a message via CAN
    #[task(binds = CAN1_RX0, shared = [can1])]
    fn can_receive(ctx: can_receive::Context) {
        let mut can1 = ctx.shared.can1;
        let frame = can1.lock(|can1| can1.receive().unwrap());

        info!(
            "Received frame with first byte: {}",
            frame.data().unwrap().first().unwrap()
        );

        can_send::spawn().ok();
    }
}
