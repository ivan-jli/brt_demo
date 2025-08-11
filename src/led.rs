
pub struct Led {
    nbr_toggles: u8,
    current_toggle: u8,
    //used to scale down the led toggling, making it blink slower
    divider: u8,
    divider_current_cycle: u8,
    // active: bool,
}

impl Led {
    pub fn new(nbr_blinks: u8, divider: u8) -> Self {
        Led { nbr_toggles: nbr_blinks * 2, current_toggle: 0, divider, divider_current_cycle: 0,
            // active: false
        }
    }
    pub fn reset(&mut self) {
        self.current_toggle = 0;
    }
    
    // pub fn activate(&mut self) {
    //     self.active = true;
    // }
    // returns TRUE if the toggling cycle has ENDED

    pub fn tick(&mut self) -> bool {
        if self.divider_current_cycle < self.divider {
            self.divider_current_cycle += 1;
        }
        else {
            self.divider_current_cycle = 0;
            if self.current_toggle < self.nbr_toggles {
                self.current_toggle += 1;
            }
            else {
                return true;
            }
        }
        false
    }
}