#![no_main]
#![no_std]

use stm32f446_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers=[USART1, USART2, USART3])]
mod app {
    use dwt_systick_monotonic::{DwtSystick, ExtU32};
    use stm32f4xx_hal::{gpio::{NoPin, self, gpioa::{PA5, PA6, PA7}, Output, PushPull}, pac::{self}, //implements pins from hal with functions needed
        prelude::*, //QOL implementation
        spi::{Mode, NoMiso, Phase, Polarity, Spi} //implementing SPI from the HAL
    };

    // Needed for scheduling monotonic tasks
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<48_000_000>; // 48 MHz

    // Needed even if we don't use it
    #[shared]
    struct Shared {
        
    }

    // Needed even if we don't use it
    #[local]
    struct Local {
        spi: Spi<pac::SPI1, (gpio::gpioa::PA5<gpio::Alternate<5>>, NoPin, gpio::gpioa::PA7<gpio::Alternate<5>>)>, //setting spi as local to be used in tasks
        cs: PA6<Output<PushPull>>,
    }


    // The init function is called in the beginning of the program
    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("init");

        // Cortex-M peripherals
        let mut _core : cortex_m::Peripherals = ctx.core;
        
        // Device specific peripherals
        let mut _device : stm32f4xx_hal::pac::Peripherals = ctx.device;

        // Set up the system clock.
        let rcc = _device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.MHz()).freeze();

        // Set up the pins.
        let gpioa = _device.GPIOA.split();
        let sclk =gpioa.pa5.into_alternate::<5>(); //stm32f446 datasheet s. 57 pdf
        let mosi =gpioa.pa7.into_alternate::<5>();
        let mut cs = gpioa.pa6.into_push_pull_output();
        cs.set_high();

        let spi_mode = stm32f4xx_hal::spi::Mode { //spi setup
            polarity: stm32f4xx_hal::spi::Polarity::IdleLow,
            phase: stm32f4xx_hal::spi::Phase::CaptureOnFirstTransition
        };

        let mut spi = _device.SPI1.spi(
            (sclk, NoMiso{}, mosi), //implementations
            spi_mode,   //ref to spi setup
            1.MHz(),    //frequency of spi
            &clocks,    //sync with system clock
            
        ); //spi definition
        
        // enable tracing and the cycle counter for the monotonic timer
        _core.DCB.enable_trace();
        _core.DWT.enable_cycle_counter();

        // Set up the monotonic timer
        let mono = DwtSystick::new(
            &mut _core.DCB,
            _core.DWT,
            _core.SYST,
            clocks.hclk().to_Hz(),
        );
        mosi::spawn_after(1.secs()).ok(); //task spawn
        (Shared {}, Local {spi, cs}, init::Monotonics(mono)) //initiation of values
        
    }


    
    // The idle function is called when there is nothing else to do
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            continue;
        }
    }
    

    // The task functions are called by the scheduler
    #[task(local=[spi, cs])]
    fn mosi(ctx: mosi::Context) {
    
        let mut data: &[u8] = "abcdef".as_bytes(); //create a list of bytes
        ctx.local.cs.set_low(); //set cs pin low
        ctx.local.spi.write(&data).unwrap(); //spi write list of bytes
        ctx.local.cs.set_high(); //set cs pin high
        
        defmt::info!("Written");
        mosi::spawn_after(5.secs()).ok(); //call task after 5 secs
    }
}
