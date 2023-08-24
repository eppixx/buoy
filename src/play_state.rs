#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum PlayState {
    Play,
    Pause,
    #[default]
    Stop,
}
