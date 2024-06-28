#[derive(Debug, Clone, Copy)]
pub enum Model {
	English = 1,
	Geology = 2,
}

impl Model {
	pub fn from_i32(value: i32) -> Option<Self> {
		match value {
			1 => Some(Model::English),
			2 => Some(Model::Geology),
			_ => None,
		}
	}
}
