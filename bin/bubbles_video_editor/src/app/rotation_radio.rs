use video_processor::RotateFlags;

#[derive(PartialEq)]
pub enum RotationRadio
{
    First(Option<RotateFlags>),
    Second(Option<RotateFlags>),
    Third(Option<RotateFlags>),
    Forth(Option<RotateFlags>),
}



impl RotationRadio 
{
    pub fn get(&self) -> Option<RotateFlags>
    {
        match self 
        {
            Self::First(value)  => *value,
            Self::Second(value) => *value,
            Self::Third(value)  => *value,
            Self::Forth(value)  => *value,
        }
    }    
}
