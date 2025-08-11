
use esp_backtrace as _;
use esp_hal::{
    time::{Duration, Instant}, timer::Timer
};

#[derive (Clone, Copy)]
struct MovementInner {
    ts_first_movement: Instant, 
    ts_last_movement: Instant, 
}

pub struct Movement {
    movement: Option<MovementInner>,
}

impl Movement {
    pub fn new() -> Self {
        Movement {movement: None }
    }
    
    // called on each IMU movement interrupt
    pub fn register_move(&mut self) {
        let ts = Instant::now();

        if let Some(mut movement) = self.movement {
            movement.ts_last_movement = ts;
        }
        else { //first movement in a series
            self.movement = Some(MovementInner { ts_first_movement: ts, ts_last_movement: ts });
        }
    }
    
    // returns true if 10s of movement are detected. Resets the 
    pub fn is_10s_movement(&mut self) -> bool {
        let ts = Instant::now();
        if let Some(movement) = self.movement {
            if movement.ts_last_movement - movement.ts_first_movement > Duration::from_secs(10) {
                return true;
            }
            else { 
                if ts - movement.ts_first_movement > Duration::from_secs(20) {
                    self.movement = None;
                }
            }
        }
        false
    }
}
