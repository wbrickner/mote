/// Representation of a model name
#[derive(Debug, Clone)]
pub struct Model {
  /// The consumer-recognizable name of the model
  pub name: String,

  /// Any internal or more technical name for this model
  pub alternate_name: String,

  /// The official model number, the most specific and technical identifier
  pub number: String
}