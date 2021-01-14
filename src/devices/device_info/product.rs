use super::Model;

#[derive(Debug, Clone)]
pub struct Product {
  pub vendor: String,
  pub model: Model,
  pub serial_number: String
}