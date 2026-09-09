#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    MoveForward,
    MoveBackward,
    StrafeLeft,
    StrafeRight,
    TurnLeft,
    TurnRight,
    Fire,
    Quit,
    EquipSlot(u8),
    SwitchWeapon,
    ScrollUp,
    ScrollDown,
    QuickMelee,
    QuickThrowGrenade,
}
