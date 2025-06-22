use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AssetSize {
    Full,
    Short
}

impl ToString for AssetSize {
    fn to_string(&self) -> String {
        match self {
            AssetSize::Full => "fulfill".into(),
            AssetSize::Short => "shortened".into()
        }
    }
}

pub enum DeviceType {
    None,
    Ios,
    Android
}

impl ToString for DeviceType {
    fn to_string(&self) -> String {
        match self {
            DeviceType::None => "0".into(),
            DeviceType::Ios => "1".into(),
            DeviceType::Android => "2".into()
        }
    }
}