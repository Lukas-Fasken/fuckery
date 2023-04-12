#![deny(unsafe_code)]
#![no_main]
#![no_std]
#![allow(unused_imports)]

use stm32f446_rtic as _; // global logger + panicking-behavior + memory layout
use rtic::app;

#[app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [USART1, USART2])]
mod app {

    use core::sync::atomic::{AtomicUsize, Ordering};
    use embedded_hal::blocking::spi::Operation::Write;
    use embedded_hal::blocking::spi::Write as SpiWrite;
    use stm32f4xx_hal::{
        gpio::{gpioa::{PA5, PA6, PA7, PA9}, Alternate, AF5, Output, PushPull, Pin, Input, Edge},
        spi::{Mode, NoMiso, Phase, Polarity, Spi},
        prelude::*,
        pac::{SPI1, Interrupt, EXTI},
    };
    use dwt_systick_monotonic::{DwtSystick, ExtU32};
    use rtic::Mutex;

    #[shared]
    struct Shared { 
    }

    #[local]
    struct Local {  
        slk: Pin<'A', 5, Output<PushPull>>,
        masterosi: Pin<'A', 7, Output<PushPull>>,
        masteriso: Pin<'A', 6, Input>,
        cs: Pin<'A', 9, Output<PushPull>>,
    }

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<48_000_000>; // 48 MHz

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("init");

        // Cortex-M peripherals
        let mut _core : cortex_m::Peripherals = ctx.core;

        // Device specific peripherals
        let mut _device : stm32f4xx_hal::pac::Peripherals = ctx.device;

        // Set up the system clock.
        let rcc = _device.RCC.constrain();
        let _clocks = rcc.cfgr.sysclk(48.MHz()).freeze();

        // enable tracing and the cycle counter for the monotonic timer
        _core.DCB.enable_trace();
        _core.DWT.enable_cycle_counter();

        //define SPI pins
        let gpioa = _device.GPIOA.split();
        let slk = gpioa.pa5.into_push_pull_output(); //sync clock speed
        let masterosi = gpioa.pa7.into_push_pull_output();
        let cs = gpioa.pa9.into_push_pull_output(); //ss -> open of transmission
        let mut masteriso = gpioa.pa6.into_pull_down_input();



        // Set up the monotonic timer
        let mono = DwtSystick::new(
            &mut _core.DCB,
            _core.DWT,
            _core.SYST,
            _clocks.hclk().to_Hz(),
        );

        transmit::spawn(170,3000,true).ok(); //10101010...

        (
            Shared { }, 
            Local {cs, masterosi, slk, masteriso}, 
            init::Monotonics(mono)
        )
    }

    // The idle function is called when there is nothing else to do
    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            continue;
        }
    }


    #[task(shared = [], local = [cs, masterosi, slk], priority = 2)]    //<----------undersøg lige hvad der skal ind i her :)
    fn transmit(_ctx: transmit::Context, reg: u8, data: u8, clk_pos: bool){

        defmt::info!("We are sending to reg: {}, with data: {}!", reg, data);
        let mut byte: u8 = data;            //dataen der skal sendes
        let mut clk_posin: bool = clk_pos;  //vi vil gerne vide hvilken pos clocken er i.


        //outputclock speed!
        if clk_pos == false {
            _ctx.local.slk.set_low();
            clk_posin = true;
            defmt::info!("clock Low");        
        }else {
            _ctx.local.slk.set_high();
            clk_posin = false;
            defmt::info!("clock High"); 
        }


        // Create the SPI message to send
        let mut tx_data: &[u8] = &[byte];

        // Send the SPI message and check if somthing has been returned.
        //(byte >> 7) & 1) tjekker den MSB og ser om den er 1.
        //(byte & 1) tjekker LSB, vi bruger ikke den her men det er en god note.
        if data != 0b0000_0000 {
            _ctx.local.cs.set_low();
            if ((byte >> 7) & 1) == 1 {
                _ctx.local.masterosi.set_high();
                //defmt::info!("High time");
                defmt::info!("bit: {}", ((byte >> 7) & 1));

            }else {
                _ctx.local.masterosi.set_low();
                //defmt::info!("Low time");
                defmt::info!("bit: {}", ((byte >> 7) & 1));                
            }

            byte <<= 1; //rykke en plads til venstre
            //tjekker om vi har fået et indput - dette går dog for hurtigt til at vi kan tjek med en stm :(
            //hvis dette går for hurtgit kan vi eventuelt lav en ny fn som spawner hver clockspeed/2.
            recieve::spawn_after(150.millis(), 6, byte, clk_posin);

            //her sender vi den næste bit efter 300 millis, vi kan gøre det hurtigere ved at sætte tiden ned.
            //jeg laver måske en program der regner millis om til hz, så det er nemmere at se hvad vores clock speed er.
            transmit::spawn_after(300.millis(),6, byte, clk_posin);
        }
        else {//når vi er done med at sende sætter vi vores cs(ss) til høj så vi fortæller slaven vi ikke vil snakke med den
            //mere. vi sætter også MOSI til lav mest fordi jeg tænker den brugre strøm når den er høj men ved det ikke  manner.
            _ctx.local.cs.set_high();
            _ctx.local.masterosi.set_low();
        } 

    }
    //den her fanger infmormationen og sætter den i et 8bit register, dog dør informationen efter den er kørt xd
    #[task(shared = [], local = [masteriso], priority = 1)]    //<----------undersøg lige hvad der skal ind i her :)
    fn recieve(_ctx: recieve::Context, reg: u8, data: u8, clk_pos: bool){
        let mut is_high:bool = _ctx.local.masteriso.is_high();  //vores input pin 
        let mut dataRecieved: u8 = 0;

            if is_high && clk_pos == true {
                defmt::info!("Bit recived! counting up by one!!");
                dataRecieved += 1;
                dataRecieved <<= 1;            
            }else {
                defmt::info!("Bit not recived! so we set 0 bit in!!");
                dataRecieved += 0;
                dataRecieved <<= 1;
        }        
    }
}