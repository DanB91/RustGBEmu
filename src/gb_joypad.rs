
//Used to tell whether a Game Boy button is up or down 
#[derive(PartialEq, Copy, Clone)]
pub enum ButtonState {
    Down = 0, //Key is currently held down (bit is turned off)
    Up = 1 //Key is unpressed (bit is turned on)
}

//The comments on each field tell which bit of the JOYP register 
//each button group corresponds to 
pub enum ButtonGroup {
    FaceButtons, //Bit 4
    DPad, //Bit 5
    Nothing //Bit 4 and 5 turned off
}

//The comments on each field tell which bit of the JOYP register 
//each button corresponds to 
pub struct JoypadState {

    //FaceButtons
    pub a: ButtonState, //Bit 0
    pub b: ButtonState, // Bit 1
    pub select: ButtonState, //Bit 2
    pub start: ButtonState, //Bit 3

    //DPad
    pub right: ButtonState, //Bit 0
    pub left: ButtonState, //Bit 1
    pub up: ButtonState, //Bit 2
    pub down: ButtonState, //Bit 3

    pub selectedButtonGroup: ButtonGroup


}

impl JoypadState {
    pub fn new() -> JoypadState {
        use self::ButtonState::*;
        use self::ButtonGroup::*;
        
        JoypadState {
            a: Up,
            b: Up,
            select: Up,
            start: Up,

            right: Up,
            left: Up,
            up: Up,
            down: Up,

            selectedButtonGroup: Nothing 
        }
    }
}

