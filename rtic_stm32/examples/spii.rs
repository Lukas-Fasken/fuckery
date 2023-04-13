#![no_main]
#![no_std]

use stm32f446_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers=[USART1, USART2, USART3])]
mod app {
    use dwt_systick_monotonic::{DwtSystick, ExtU32};
    use stm32f4xx_hal::{gpio::{NoPin, self}, pac::{self},
        prelude::*, 
        spi::{Mode, NoMiso, Phase, Polarity, Spi}          
    };

    // Needed for scheduling monotonic tasks
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<48_000_000>; // 48 MHz

    // Holds the shared resources (used by multiple tasks)
    // Needed even if we don't use it
    #[shared]
    struct Shared {
        
    }

    // Holds the local resources (used by a single task)
    // Needed even if we don't use it
    #[local]
    struct Local {
        spi: Spi<pac::SPI1, (gpio::gpioa::PA5<gpio::Alternate<5>>, NoPin, gpio::gpioa::PA7<gpio::Alternate<5>>)>,
    }


    // The init function is called in the beginning of the program
    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("init");

        //let device = pac::Peripherals::take().unwrap();   // added for spi
        //let rcc = device.RCC.constrain();                         // added for spi
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





        let spi_mode = stm32f4xx_hal::spi::Mode {
            polarity: stm32f4xx_hal::spi::Polarity::IdleLow,
            phase: stm32f4xx_hal::spi::Phase::CaptureOnFirstTransition
        };

        let spi = _device.SPI1.spi(
            (sclk, NoMiso{}, mosi),
            spi_mode,
            1.MHz().into(),
            &clocks,
        );
        

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
        mosi::spawn_after(1.secs()).ok();
        (Shared {}, Local {spi}, init::Monotonics(mono))
    }


    
    // The idle function is called when there is nothing else to do
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            continue;
        }
    }
    

    // The task functions are called by the scheduler
    #[task(local=[spi])]
    fn mosi(ctx: mosi::Context) {
        //let spi =ctx.resources.spi;
        let data: &[u8] = "hej".as_bytes();
        ctx.local.spi.write(&data).unwrap();
        defmt::info!("value: {}", data);
        defmt::info!("Written");
        //let res = ctx.local.spi.read().unwrap();
    }
}
