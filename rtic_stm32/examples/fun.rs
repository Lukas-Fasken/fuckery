#![no_main]
#![no_std]

use stm32f446_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [USART1, USART2])]
mod app {
    use dwt_systick_monotonic::{DwtSystick, ExtU32};
    use stm32f4xx_hal::{
        gpio::{gpioa::PA5,gpioa::PA1, Input, Output, PushPull},
        prelude::*,
    };

    // Needed for scheduling monotonic tasks
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<48_000_000>; // 48 MHz

    // Holds the shared resources (used by multiple tasks)
    // Needed even if we don't use it

    #[shared]
    struct Shared {
        global: u32,
    }

    // Holds the local resources (used by a single task)
    // Needed even if we don't use it
    #[local]
    struct Local {
        led: PA5<Output<PushPull>>,
        ex_led: PA1<Output<PushPull>>,
        //button: PA4<Input<PushPull>>,
        a: u32,
        b: u32,
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

        // Set up the LED. On the Nucleo-F446RE it's connected to pin PA5. and external led
        let gpioa = _device.GPIOA.split();
        let led = gpioa.pa5.into_push_pull_output();
        let ex_led = gpioa.pa1.into_push_pull_output();
        //let button = gpioa.pa4.into_push_pull_input();

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

        defmt::info!("Init done!");
        //spawn the functions
        blink::spawn_after(1.secs()).ok();
        add::spawn_after(2.secs()).ok();
        (Shared {global:0}, Local { led, ex_led, a:0, b:0 }, init::Monotonics(mono))
    }

    // The idle function is called when there is nothing else to do
    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("idle");
        loop {
            continue;
        }
    }



    #[task]
    fn fancy(ctx: fancy::Context){
        defmt::info!("this task has run");
    }



    // The task functions are called by the scheduler
    #[task(priority=2, shared=[global], local = [led, ex_led])]
    fn blink(mut ctx: blink::Context) {                 //changing predefined values add mut to ctx
        ctx.local.led.toggle();                         //toggle internal led
        ctx.local.ex_led.toggle();
        ctx.shared.global.lock(|global| *global +=1);   //lock and increment global
        defmt::info!("Blink!");
        fancy::spawn().ok();
        blink::spawn_after(1.secs()).ok();
    }



    #[task(priority=1, shared=[global], local=[a, b])]
    fn add(mut ctx: add::Context) {                 //changing predefined values add mut to ctx
    defmt::info!("task2");
    ctx.shared.global.lock(|global| *global +=1);   //lock and increment global
    let mut d:u32=0;                                //needs to be equal to something, type is not needed
    ctx.shared.global.lock(|global| d=*global);     //lock and assign d with global value
    *ctx.local.a+=1;
    *ctx.local.b+=1;
    let result = *ctx.local.a + *ctx.local.b;
    defmt::info!("global: {}", d);
    //defmt::info!("Result: {}", result);
    add::spawn_after(2.secs()).ok();
}
    
}
    