pub enum VideoMode
{
    Play(&'static str),
    Pause(&'static str),
}
impl VideoMode
{
    pub const PLAY: VideoMode  = VideoMode::Play("Play");
    pub const PAUSE: VideoMode = VideoMode::Pause("Pause");
    pub fn get_name(&self) -> &'static str 
    {
        match self
        {
            VideoMode::Pause(s) => s,
            VideoMode::Play(s) => s,
        }
    }    
}
