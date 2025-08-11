#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

// use core::time::Duration;

use esp_backtrace as _;
use cassette::Cassette;
use esp_hal::{
    delay::Delay, gpio::{Event, Input, InputConfig, Io, Pull, WakeEvent}, handler, i2c::master::*, main, rtc_cntl::{
        reset_reason, sleep::{GpioWakeupSource, RtcSleepConfig, TimerWakeupSource, WakeSource, WakeTriggers, WakeupLevel}, wakeup_cause, Rtc, Rwdt, RwdtStage, SocResetReason
        
    }, system::Cpu, time::{Duration, Instant}, timer::{timg::TimerGroup, PeriodicTimer, Timer}, DriverMode
};
use esp_println::println;
use critical_section::Mutex;
use core::cell::RefCell;
use lis3dh_async::{Lis3dh, SlaveAddr, IrqPin1Config, Interrupt1, InterruptConfig, InterruptMode};
use brt_demo::movement::*;
use brt_demo::led::*;

static ACCEL_INT: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
//RWDT usage example from esp_hal/rtc_cntl
static RWDT: Mutex<RefCell<Option<Rwdt>>> = Mutex::new(RefCell::new(None));
static HALL_INT: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

static MOVEMENT: Mutex<RefCell<Option<Movement>>> = Mutex::new(RefCell::new(None));
static STATE: Mutex<RefCell<Option<State>>> = Mutex::new(RefCell::new(Some(State::Sleeping)));
static ACCEL_LED: Mutex<RefCell<Option<Led>>> = Mutex::new(RefCell::new(None));

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    let delay = Delay::new();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    // Setting ISR handlers
    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(gpio_handler);

    // Setting i2c
    let i2c = I2c::new(peripherals.I2C0, Config::default()).unwrap().into_async()
    .with_sda(peripherals.GPIO1)
    .with_scl(peripherals.GPIO2);
    
    // Setting Timer
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut timer0 = PeriodicTimer::new(timg0.timer0);
    // timer0.start(Duration::from_secs(1)).unwrap();
    // let now = timer0.now();
    // timer0.load_value(Duration::from_secs(1)).unwrap();
    timer0.set_interrupt_handler(rtc_handler);
    timer0.enable_interrupt(true);
    // timer0.start();
    
    // Setting RWDT
    // timg0.rwdt.set_timeout(RwdtStage::Stage0, Duration::from_millis(10));
    // timg0.rwdt.listen();
    // critical_section::with(|cs| RWDT.borrow_ref_mut(cs).replace(timer0));
    
    // Where the `LP_WDT` interrupt handler is defined as:
    // static RWDT: Mutex<RefCell<Option<Rwdt>>> = Mutex::new(RefCell::new(None));

    // Setting GPIO
    let accel_int = peripherals.GPIO9; //GPIO9 == BOOT Button
    let pin3 = peripherals.GPIO3;
    let mut accel_int_in = Input::new(
        accel_int,
        InputConfig::default()
        // .with_pull(Pull::Down), //LIS3DH 38/54 DocID17530 Rev 2: default INT_POLARITY 0 - active high
        .with_pull(Pull::Up), 
    );
    delay.delay_millis(100); // wait for the pullups to charge any capacitence
    
    // Setting accelerometer
    // let x = core::pin::pin!(accel_init(i2c));
    // let cm = Cassette::new(x);
    // cm.block_on();

    println!("Init successful.");
    let reason = reset_reason(Cpu::ProCpu).unwrap_or(SocResetReason::ChipPowerOn);
    println!("Reset reason: {:?}", reason);

    // let timer = TimerWakeupSource::new(CoreDuration::from_secs(10));

    // core::mem::drop(accel_int_in);

    let mut rtc_lpwr = Rtc::new(peripherals.LPWR);
    // let gpio_wakeup_source = GpioWakeupSource::default();
    let gpio_wakeup_source = GpioWakeupSource::new();
    
    let mut wake_triggers = WakeTriggers::default();
    wake_triggers.set_gpio(true);

    let mut sleep_config = RtcSleepConfig::default();
    // sleep_config.wifi_pd_en()

    gpio_wakeup_source.apply(&rtc_lpwr, &mut wake_triggers, &mut sleep_config);

    // gpio_wakeup_source.apply(&rtc_lpwr, sleep_config).wakeup_enable(true, esp_hal::gpio::WakeEvent::HighLevel).unwrap(); // wakeup_enable unlistens for interrupts
    // gpio_wakeup_source.apply(&rtc_lpwr, triggers, sleep_config); //todo?

    
    let wake_reason = wakeup_cause();
    println!("wake reason: {:?}", wake_reason);
    
    // commenting out accelerometer init in order to test on esp hardware without that sensor
    // critical_section::with(|cs| {
    //     accel_int_in.listen(Event::LowLevel);
    //     ACCEL_INT.borrow_ref_mut(cs).replace(accel_int_in);
    // });
     
    // LEDs
    let accel_led = Led::new(2, 4);
    critical_section::with(|cs| {
        ACCEL_LED.borrow_ref_mut(cs).replace(accel_led);
    });

    accel_int_in.wakeup_enable(true, WakeEvent::LowLevel).unwrap();
    // accel_int_in.wakeup_enable(true, WakeEvent::HighLevel).unwrap();
    println!("Going to light sleep!");
    delay.delay_millis(100);
    rtc_lpwr.sleep_light(&[&gpio_wakeup_source]); // 250810 2003 gpio_wakeup_source isn't correct
    delay.delay_millis(100);
    println!("exiting light sleep!");
    
    let mut counter: u32 = 0;
    // let mut state = State::default();
    loop {
        counter += 1;
        if counter % 50_000_000 == 0 {
            println!(".");
        }

    // critical_section::with(|cs| {
    //     let mut a = HALL_INT.borrow_ref_mut(cs);
    //     let Some(a) = a.as_mut() else {
    //         return;
    //     };
    //     // if a.
    // });
        
    }
}

//using the example from esp_hal::gpio::Input listen()
#[handler]
fn gpio_handler() {
    critical_section::with(|cs| {
        let mut accel_int = ACCEL_INT.borrow_ref_mut(cs);
        let Some(accel_int) = accel_int.as_mut() else {
            // Some other interrupt has occurred
            // before the button was set up.
            return;
        };

        if accel_int.is_interrupt_set() {
            accel_int.clear_interrupt();
            println!("A ISR!");

            let mut movement = MOVEMENT.borrow_ref_mut(cs);
            if let Some(m) = movement.as_mut() {
                m.register_move();
            };
            // accel_int.unlisten();
            // loop {};
        }
    });
}


//periodic interrupt each 100 ms, when the uC is not sleeping
#[handler]
fn rtc_handler() { 
    critical_section::with(|cs| {
        println!("rtc ISR");
        let mut a = RWDT.borrow_ref_mut(cs);
        let Some(a) = a.as_mut() else {
            return;
        };
        a.clear_interrupt();
        // a.unlisten();
        let mut a = MOVEMENT.borrow_ref_mut(cs);
        if let Some(movement) = a.as_mut() {
            Instant::now();
        }

        let mut state = STATE.borrow_ref_mut(cs);
        let state = state.as_mut();
        if let Some(state) = state {
            match state {
                State::Sleeping => {
                    //this is reached right after going out of sleep
                    
                },
                State::Movement => {
                    let mut movement = MOVEMENT.borrow_ref_mut(cs);
                    if let Some(movement) = movement.as_mut() {
                        if movement.is_10s_movement() {
                            *state = State::Movement10sIndication;
                            let mut accel_led = ACCEL_LED.borrow_ref_mut(cs);
                            if let Some(accel_led) = accel_led.as_mut() {
                                accel_led.reset();
                            }
                        }
                    }
                    else {
                        *movement = Some(Movement::new());
                    }
                    
                },
                State::Movement10sIndication => {
                    let mut accel_led = ACCEL_LED.borrow_ref_mut(cs);
                    if let Some(accel_led) = accel_led.as_mut() {
                        accel_led.tick();
                    }
                },
                State::HallSensorStateChangeIndication => todo!(),
            }

        }

    });
}

#[handler]
fn gpio_hall_sensor_handler() {
    critical_section::with(|cs| {
        let mut a = HALL_INT.borrow_ref_mut(cs);
        let Some(a) = a.as_mut() else {
            return;
        };
        a.clear_interrupt();
        let mut state = STATE.borrow_ref_mut(cs);
        // *state = State::HallSensorStateChangeIndication;

    });
}


async fn accel_init<'a, Dm: DriverMode>(i2c: I2c<'a, Dm>) 
// -> Lis3dh<lis3dh_async::Lis3dhI2C<I2c<'a, Dm>>>
where esp_hal::i2c::master::I2c<'a, Dm>: embedded_hal_async::i2c::I2c {
    let mut lis3dh = Lis3dh::new_i2c(i2c, SlaveAddr::Alternate).await.unwrap();
    lis3dh.configure_interrupt_pin(IrqPin1Config {
        // Raise if interrupt 1 is raised
        ia1_en: true,
        // Disable for all other interrupts
        ..IrqPin1Config::default()
    }).await.unwrap();
    lis3dh.configure_irq_src(Interrupt1, InterruptMode::Movement, 
        InterruptConfig::high() 
        // { z_axis_high: (), z_axis_low: (), y_axis_high: (), y_axis_low: (), x_axis_high: (), x_axis_low: () }
    ).await.unwrap();
}

enum State {
    Sleeping,
    Movement,
    Movement10sIndication,
    HallSensorStateChangeIndication,
}

// impl Default for State {
//     fn default() -> Self {
//         State::Sleeping
//     }
// }