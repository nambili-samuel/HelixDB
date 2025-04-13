use std::collections::HashSet;

use super::parser_methods::ParserError;
use crate::protocol::value::Value;
use pest::{
    iterators::{Pair, Pairs},
    Parser as PestParser,
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct HelixParser {
    source: Source,
}

impl Default for HelixParser {
    fn default() -> Self {
        HelixParser {
            source: Source {
                node_schemas: Vec::new(),
                edge_schemas: Vec::new(),
                vector_schemas: Vec::new(),
                queries: Vec::new(),
            },
        }
    }
}

// AST Structures
#[derive(Debug, Clone)]
pub struct Source {
    pub node_schemas: Vec<NodeSchema>,
    pub edge_schemas: Vec<EdgeSchema>,
    pub vector_schemas: Vec<VectorSchema>,
    pub queries: Vec<Query>,
}

#[derive(Debug, Clone)]
pub struct NodeSchema {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct VectorSchema {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct EdgeSchema {
    pub name: String,
    pub from: String,
    pub to: String,
    pub properties: Option<Vec<Field>>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    Array(Box<FieldType>),
    Identifier(String),
}

#[derive(Debug, Clone)]
pub struct Query {
    pub original_query: String,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub statements: Vec<Statement>,
    pub return_values: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: FieldType,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment(Assignment),
    AddVector(AddVector),
    AddNode(AddNode),
    AddEdge(AddEdge),
    Drop(Expression),
    SearchVector(SearchVector),
    BatchAddVector(BatchAddVector),
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub variable: String,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Traversal(Box<Traversal>),
    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(i32),
    FloatLiteral(f64),
    BooleanLiteral(bool),
    Exists(Box<Traversal>),
    BatchAddVector(BatchAddVector),
    AddVector(AddVector),
    AddNode(AddNode),
    AddEdge(AddEdge),
    And(Vec<Expression>),
    Or(Vec<Expression>),
    SearchVector(SearchVector),
    None,
}

#[derive(Debug, Clone)]
pub struct Traversal {
    pub start: StartNode,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct BatchAddVector {
    pub vector_type: Option<String>,
    pub vec_identifier: Option<String>,
}

#[derive(Debug, Clone)]
pub enum StartNode {
    Node {
        types: Option<Vec<String>>,
        ids: Option<Vec<String>>,
    },
    Edge {
        types: Option<Vec<String>>,
        ids: Option<Vec<String>>,
    },
    Variable(String),
    Anonymous,
}

#[derive(Debug, Clone)]
pub enum Step {
    Node(GraphStep),
    Edge(GraphStep),
    Where(Box<Expression>),
    BooleanOperation(BooleanOp),
    Count,
    Update(Update),
    Object(Object),
    Exclude(Exclude),
    Closure(Closure),
    Range((Expression, Expression)),
    AddEdge(AddEdge),
    SearchVector(String),
}

#[derive(Debug, Clone)]
pub struct FieldAddition {
    pub name: String,
    pub value: FieldValue,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Traversal(Box<Traversal>),
    Expression(Expression),
    Fields(Vec<FieldAddition>),
    Literal(Value),
    Empty,
}

#[derive(Debug, Clone)]
pub enum GraphStep {
    Out(Option<Vec<String>>),
    In(Option<Vec<String>>),
    Both(Option<Vec<String>>),
    OutN,
    InN,
    BothN,
    OutE(Option<Vec<String>>),
    InE(Option<Vec<String>>),
    BothE(Option<Vec<String>>),
}

#[derive(Debug, Clone)]
pub enum BooleanOp {
    And(Vec<Expression>),
    Or(Vec<Expression>),
    GreaterThan(Box<Expression>),
    GreaterThanOrEqual(Box<Expression>),
    LessThan(Box<Expression>),
    LessThanOrEqual(Box<Expression>),
    Equal(Box<Expression>),
    NotEqual(Box<Expression>),
}

#[derive(Debug, Clone)]
pub struct AddVector {
    pub vector_type: Option<String>,
    pub data: Option<VectorData>,
}

#[derive(Debug, Clone)]
pub enum VectorData {
    Vector(Vec<f64>),
    Identifier(String),
}

#[derive(Debug, Clone)]
pub struct SearchVector {
    pub vector_type: Option<String>,
    pub data: Option<VectorData>,
    pub k: Option<EvaluatesToNumber>,
}

#[derive(Debug, Clone)]
pub enum EvaluatesToNumber {
    Integer(usize),
    Float(f64),
    Identifier(String),
}

#[derive(Debug, Clone)]
pub struct AddNode {
    pub vertex_type: Option<String>,
    pub fields: Option<Vec<(String, ValueType)>>,
}

#[derive(Debug, Clone)]
pub struct AddEdge {
    pub edge_type: Option<String>,
    pub fields: Option<Vec<(String, ValueType)>>,
    pub connection: EdgeConnection,
    pub from_identifier: bool,
}

#[derive(Debug, Clone)]
pub struct EdgeConnection {
    pub from_id: Option<IdType>,
    pub to_id: Option<IdType>,
}

#[derive(Debug, Clone)]
pub enum IdType {
    Literal(String),
    Identifier(String),
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Literal(Value),
    Identifier(String),
    Object(Object),
}
impl From<Value> for ValueType {
    fn from(value: Value) -> ValueType {
        match value {
            Value::String(s) => ValueType::Literal(Value::String(s)),
            Value::Integer(i) => ValueType::Literal(Value::Integer(i)),
            Value::Float(f) => ValueType::Literal(Value::Float(f)),
            Value::Boolean(b) => ValueType::Literal(Value::Boolean(b)),
            Value::Array(arr) => ValueType::Literal(Value::Array(arr)),
            Value::Empty => ValueType::Literal(Value::Empty),
            _ => unreachable!(),
        }
    }
}

impl From<IdType> for String {
    fn from(id_type: IdType) -> String {
        match id_type {
            IdType::Literal(mut s) => {
                s.retain(|c| c != '"');
                s
            }
            IdType::Identifier(s) => s,
        }
    }
}

impl From<String> for IdType {
    fn from(mut s: String) -> IdType {
        s.retain(|c| c != '"');
        IdType::Literal(s)
    }
}

#[derive(Debug, Clone)]
pub struct Update {
    pub fields: Vec<FieldAddition>,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub fields: Vec<(String, FieldValue)>,
    pub should_spread: bool,
}

#[derive(Debug, Clone)]
pub struct Exclude {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub identifier: String,
    pub object: Object,
}

impl HelixParser {
    pub fn parse_source(input: &str) -> Result<Source, ParserError> {
        let file = match HelixParser::parse(Rule::source, input) {
            Ok(mut pairs) => {
                let pair = pairs
                    .next()
                    .ok_or_else(|| ParserError::from("Empty input"))?;
                pair
            }
            Err(e) => {
                return Err(ParserError::from(e));
            }
        };

        let mut parser = HelixParser {
            source: Source {
                node_schemas: Vec::new(),
                edge_schemas: Vec::new(),
                vector_schemas: Vec::new(),
                queries: Vec::new(),
            },
        };

        let pairs = file.into_inner();
        let mut remaining = HashSet::new();
        for pair in pairs {
            match pair.as_rule() {
                Rule::node_def => parser
                    .source
                    .node_schemas
                    .push(parser.parse_node_def(pair)?),
                Rule::edge_def => parser
                    .source
                    .edge_schemas
                    .push(parser.parse_edge_def(pair)?),
                Rule::vector_def => parser
                    .source
                    .vector_schemas
                    .push(parser.parse_vector_def(pair)?),
                Rule::query_def => {
                    // parser.source.queries.push(parser.parse_query_def(pairs.next().unwrap())?),
                    remaining.insert(pair);
                }
                Rule::EOI => (),
                _ => return Err(ParserError::from("Unexpected rule encountered")),
            }
        }
        for pair in remaining {
            // println!("{:?}", parser.source);
            parser.source.queries.push(parser.parse_query_def(pair)?);
        }

        // parse all schemas first then parse queries using self

        Ok(parser.source)
    }

    fn parse_field_defs(&self, pair: Pair<Rule>) -> Result<Vec<Field>, ParserError> {
        pair.into_inner()
            .map(|p| self.parse_field_def(p))
            .collect::<Result<Vec<_>, _>>()
    }

    fn parse_node_def(&self, pair: Pair<Rule>) -> Result<NodeSchema, ParserError> {
        let mut pairs = pair.into_inner();
        let name = pairs.next().unwrap().as_str().to_string();
        let fields = self.parse_node_body(pairs.next().unwrap())?;
        Ok(NodeSchema { name, fields })
    }
    fn parse_vector_def(&self, pair: Pair<Rule>) -> Result<VectorSchema, ParserError> {
        let mut pairs = pair.into_inner();
        let name = pairs.next().unwrap().as_str().to_string();
        Ok(VectorSchema { name })
    }
    fn parse_node_body(&self, pair: Pair<Rule>) -> Result<Vec<Field>, ParserError> {
        let field_defs = pair
            .into_inner()
            .find(|p| p.as_rule() == Rule::field_defs)
            .expect("Expected field_defs in properties");

        // Now parse each individual field_def
        field_defs
            .into_inner()
            .map(|p| self.parse_field_def(p))
            .collect::<Result<Vec<_>, _>>()
    }

    fn parse_field_type(
        &self,
        type_str: &str,
        schema: Option<&Source>,
    ) -> Result<FieldType, ParserError> {
        match type_str {
            "String" => Ok(FieldType::String),
            "Integer" => Ok(FieldType::Integer),
            "Float" => Ok(FieldType::Float),
            "Boolean" => Ok(FieldType::Boolean),
            _ if type_str.starts_with("[") && type_str.ends_with("]") => {
                return Ok(FieldType::Array(Box::new(
                    self.parse_field_type(&type_str[1..type_str.len() - 1], schema)?,
                )));
            }
            _ if type_str.starts_with(
                |c: char| {
                    if c.is_ascii_uppercase() {
                        true
                    } else {
                        false
                    }
                },
            ) =>
            {
                // println!("{:?}", self.source);
                if self.source.edge_schemas.iter().any(|e| {
                    if e.name == type_str {
                        true
                    } else {
                        false
                    }
                }) || self.source.node_schemas.iter().any(|n| {
                    if n.name == type_str {
                        true
                    } else {
                        false
                    }
                }) {
                    Ok(FieldType::Identifier(type_str.to_string()))
                } else {
                    return Err(ParserError::ParamDoesNotMatchSchema(type_str.to_string()));
                }
            }
            _ => unreachable!(),
        }
    }

    fn parse_field_def(&self, pair: Pair<Rule>) -> Result<Field, ParserError> {
        let mut pairs = pair.into_inner();
        let name = pairs.next().unwrap().as_str().to_string();

        let field_type =
            self.parse_field_type(pairs.next().unwrap().as_str(), Some(&self.source))?;

        Ok(Field { name, field_type })
    }

    fn parse_edge_def(&self, pair: Pair<Rule>) -> Result<EdgeSchema, ParserError> {
        let mut pairs = pair.into_inner();
        let name = pairs.next().unwrap().as_str().to_string();
        let body = pairs.next().unwrap();
        let mut body_pairs = body.into_inner();

        let from = body_pairs.next().unwrap().as_str().to_string();
        let to = body_pairs.next().unwrap().as_str().to_string();
        let properties = Some(self.parse_properties(body_pairs.next().unwrap())?);

        Ok(EdgeSchema {
            name,
            from,
            to,
            properties,
        })
    }
    fn parse_properties(&self, pair: Pair<Rule>) -> Result<Vec<Field>, ParserError> {
        pair.into_inner()
            .find(|p| p.as_rule() == Rule::field_defs)
            .map_or(Ok(Vec::new()), |field_defs| {
                field_defs
                    .into_inner()
                    .map(|p| self.parse_field_def(p))
                    .collect::<Result<Vec<_>, _>>()
            })
    }

    fn parse_query_def(&self, pair: Pair<Rule>) -> Result<Query, ParserError> {
        let original_query = pair.clone().as_str().to_string();
        let mut pairs = pair.into_inner();
        let name = pairs.next().unwrap().as_str().to_string();
        let parameters = self.parse_parameters(pairs.next().unwrap())?;
        let nect = pairs.next().unwrap();
        let statements = self.parse_query_body(nect)?;
        let return_values = self.parse_return_statement(pairs.next().unwrap())?;

        Ok(Query {
            name,
            parameters,
            statements,
            return_values,
            original_query,
        })
    }

    fn parse_parameters(&self, pair: Pair<Rule>) -> Result<Vec<Parameter>, ParserError> {
        let mut seen = HashSet::new();
        pair.clone()
            .into_inner()
            .map(|p: Pair<'_, Rule>| -> Result<Parameter, ParserError> {
                let mut inner = p.into_inner();
                let name = inner.next().unwrap().as_str().to_string();
                let param_type =
                    self.parse_field_type(inner.next().unwrap().as_str(), Some(&self.source))?;
                if seen.insert(name.clone()) {
                    Ok(Parameter { name, param_type })
                } else {
                    Err(ParserError::from(format!(
                        r#"Duplicate parameter name: {}
                            Please use unique parameter names.

                            Error happened at line {} column {} here: {}
                        "#,
                        name,
                        pair.line_col().0,
                        pair.line_col().1,
                        pair.as_str(),
                    )))
                }
            })
            .collect::<Result<Vec<_>, _>>()
    }

    fn parse_query_body(&self, pair: Pair<Rule>) -> Result<Vec<Statement>, ParserError> {
        pair.into_inner()
            .map(|p| match p.as_rule() {
                Rule::get_stmt => Ok(Statement::Assignment(self.parse_get_statement(p)?)),
                Rule::AddN => Ok(Statement::AddNode(self.parse_add_vertex(p)?)),
                Rule::AddV => Ok(Statement::AddVector(self.parse_add_vector(p)?)),
                Rule::AddE => Ok(Statement::AddEdge(self.parse_add_edge(p, false)?)),
                Rule::drop => Ok(Statement::Drop(self.parse_expression(p)?)),
                Rule::BatchAddV => Ok(Statement::BatchAddVector(self.parse_batch_add_vector(p)?)),
                Rule::search_vector => Ok(Statement::SearchVector(self.parse_search_vector(p)?)),
                _ => Err(ParserError::from(format!(
                    "Unexpected statement type in query body: {:?}",
                    p.as_rule()
                ))),
            })
            .collect()
    }

    fn parse_batch_add_vector(&self, pair: Pair<Rule>) -> Result<BatchAddVector, ParserError> {
        let mut vector_type = None;
        let mut vec_identifier = None;
        
        for p in pair.into_inner() {
            match p.as_rule() {
                Rule::identifier_upper => {
                    vector_type = Some(p.as_str().to_string());
                }
                Rule::identifier => {
                    vec_identifier = Some(p.as_str().to_string());
                }
                _ => {
                    return Err(ParserError::from(format!(
                        "Unexpected rule in AddV: {:?} => {:?}",
                        p.as_rule(),
                        p,
                    )))
                }
            }
        }

        Ok(BatchAddVector { vector_type, vec_identifier })
    }

    fn parse_add_vector(&self, pair: Pair<Rule>) -> Result<AddVector, ParserError> {
        let mut vector_type = None;
        let mut data = None;

        for p in pair.into_inner() {
            match p.as_rule() {
                Rule::identifier_upper => {
                    vector_type = Some(p.as_str().to_string());
                }
                Rule::vector_data => match p.clone().into_inner().next().unwrap().as_rule() {
                    Rule::identifier => {
                        data = Some(VectorData::Identifier(p.as_str().to_string()));
                    }
                    Rule::vec_literal => {
                        data = Some(VectorData::Vector(self.parse_vec_literal(p)?));
                    }
                    _ => unreachable!(),
                },
                _ => {
                    return Err(ParserError::from(format!(
                        "Unexpected rule in AddV: {:?} => {:?}",
                        p.as_rule(),
                        p,
                    )))
                }
            }
        }

        Ok(AddVector { vector_type, data })
    }

    fn parse_search_vector(&self, pair: Pair<Rule>) -> Result<SearchVector, ParserError> {
        let mut vector_type = None;
        let mut data = None;
        let mut k = None;
        for p in pair.into_inner() {
            match p.as_rule() {
                Rule::identifier_upper => {
                    vector_type = Some(p.as_str().to_string());
                }
                Rule::vector_data => match p.clone().into_inner().next().unwrap().as_rule() {
                    Rule::identifier => {
                        data = Some(VectorData::Identifier(p.as_str().to_string()));
                    }
                    Rule::vec_literal => {
                        data = Some(VectorData::Vector(self.parse_vec_literal(p)?));
                    }
                    _ => unreachable!(),
                },
                Rule::evaluates_to_number => match p.clone().into_inner().next().unwrap().as_rule() {
                    Rule::integer => {
                        k = Some(EvaluatesToNumber::Integer(p.as_str().to_string().parse::<usize>().map_err(|_| ParserError::from("Invalid integer value"))?));
                    }
                    Rule::float => {
                        k = Some(EvaluatesToNumber::Float(p.as_str().to_string().parse::<f64>().map_err(|_| ParserError::from("Invalid float value"))?));
                    }
                    Rule::identifier => {
                        k = Some(EvaluatesToNumber::Identifier(p.as_str().to_string()));
                    }
                    _ => unreachable!(),
                },
                _ => {
                    return Err(ParserError::from(format!(
                        "Unexpected rule in AddV: {:?} => {:?}",
                        p.as_rule(),
                        p,
                    )))
                }
            }
        }

        Ok(SearchVector { vector_type, data, k})
    }

    fn parse_vec_literal(&self, pair: Pair<Rule>) -> Result<Vec<f64>, ParserError> {
        let mut pairs = pair.into_inner();
        let mut vec = Vec::new();
        for p in pairs {
            vec.push(
                p.as_str()
                    .parse::<f64>()
                    .map_err(|_| ParserError::from("Invalid float value"))?,
            );
        }
        Ok(vec)
    }

    fn parse_add_vertex(&self, pair: Pair<Rule>) -> Result<AddNode, ParserError> {
        let mut vertex_type = None;
        let mut fields = None;

        for p in pair.into_inner() {
            match p.as_rule() {
                Rule::identifier_upper => {
                    vertex_type = Some(p.as_str().to_string());
                }
                Rule::create_field => {
                    fields = Some(self.parse_property_assignments(p)?);
                }
                _ => {
                    return Err(ParserError::from(format!(
                        "Unexpected rule in AddV: {:?} => {:?}",
                        p.as_rule(),
                        p,
                    )))
                }
            }
        }

        Ok(AddNode {
            vertex_type,
            fields,
        })
    }

    fn parse_property_assignments(
        &self,
        pair: Pair<Rule>,
    ) -> Result<Vec<(String, ValueType)>, ParserError> {
        pair.into_inner()
            .map(|p| {
                let mut pairs = p.into_inner();
                let prop_key = pairs
                    .next()
                    .ok_or_else(|| ParserError::from("Missing property key"))?
                    .as_str()
                    .to_string();

                let prop_val = match pairs.next() {
                    Some(p) => {
                        let value_pair = p
                            .into_inner()
                            .next()
                            .ok_or_else(|| ParserError::from("Empty property value"))?;

                        match value_pair.as_rule() {
                            Rule::string_literal => Ok(ValueType::from(Value::from(
                                value_pair.as_str().to_string(),
                            ))),
                            Rule::integer => value_pair
                                .as_str()
                                .parse()
                                .map(|i| ValueType::from(Value::Integer(i)))
                                .map_err(|_| ParserError::from("Invalid integer value")),
                            Rule::float => value_pair
                                .as_str()
                                .parse()
                                .map(|f| ValueType::from(Value::Float(f)))
                                .map_err(|_| ParserError::from("Invalid float value")),
                            Rule::boolean => Ok(ValueType::from(Value::Boolean(
                                value_pair.as_str() == "true",
                            ))),
                            Rule::identifier => {
                                Ok(ValueType::Identifier(value_pair.as_str().to_string()))
                            }
                            _ => Err(ParserError::from("Invalid property value type")),
                        }?
                    }
                    None => ValueType::from(Value::Empty),
                };

                Ok((prop_key, prop_val))
            })
            .collect()
    }

    fn parse_add_edge(
        &self,
        pair: Pair<Rule>,
        from_identifier: bool,
    ) -> Result<AddEdge, ParserError> {
        let mut edge_type = None;
        let mut fields = None;
        let mut connection = None;

        for p in pair.into_inner() {
            match p.as_rule() {
                Rule::identifier_upper => {
                    edge_type = Some(p.as_str().to_string());
                }
                Rule::create_field => {
                    fields = Some(self.parse_property_assignments(p)?);
                }
                Rule::to_from => {
                    connection = Some(self.parse_to_from(p)?);
                }
                _ => {
                    return Err(ParserError::from(format!(
                        "Unexpected rule in AddE: {:?}",
                        p.as_rule()
                    )))
                }
            }
        }

        Ok(AddEdge {
            edge_type,
            fields,
            connection: connection.ok_or_else(|| ParserError::from("Missing edge connection"))?,
            from_identifier,
        })
    }

    fn parse_id_args(&self, pair: Pair<Rule>) -> Result<Option<IdType>, ParserError> {
        let p = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::from("Missing ID"))?;
        match p.as_rule() {
            Rule::identifier => Ok(Some(IdType::Identifier(p.as_str().to_string()))),
            Rule::string_literal | Rule::inner_string => {
                Ok(Some(IdType::from(p.as_str().to_string())))
            }
            _ => Err(ParserError::from(format!(
                "Unexpected rule in parse_id_args: {:?}",
                p.as_rule()
            ))),
        }
    }

    fn parse_to_from(&self, pair: Pair<Rule>) -> Result<EdgeConnection, ParserError> {
        let mut pairs = pair.into_inner();
        let mut from_id = None;
        let mut to_id = None;
        // println!("pairs: {:?}", pairs);
        for p in pairs {
            match p.as_rule() {
                Rule::from => {
                    from_id = self.parse_id_args(p.into_inner().next().unwrap())?;
                }
                Rule::to => {
                    to_id = self.parse_id_args(p.into_inner().next().unwrap())?;
                }
                _ => unreachable!(),
            }
        }
        Ok(EdgeConnection {
            from_id: from_id,
            to_id: to_id,
        })
    }

    fn parse_get_statement(&self, pair: Pair<Rule>) -> Result<Assignment, ParserError> {
        let mut pairs = pair.into_inner();
        let variable = pairs.next().unwrap().as_str().to_string();
        let value = self.parse_expression(pairs.next().unwrap())?;

        Ok(Assignment { variable, value })
    }

    fn parse_return_statement(&self, pair: Pair<Rule>) -> Result<Vec<Expression>, ParserError> {
        // println!("pair: {:?}", pair.clone().into_inner());
        pair.into_inner()
            .map(|p| self.parse_expression(p))
            .collect()
    }

    fn parse_expression_vec(&self, pairs: Pairs<Rule>) -> Result<Vec<Expression>, ParserError> {
        let mut expressions = Vec::new();
        for p in pairs {
            match p.as_rule() {
                Rule::anonymous_traversal => {
                    expressions.push(Expression::Traversal(Box::new(
                        self.parse_anon_traversal(p)?,
                    )));
                }
                Rule::traversal => {
                    expressions.push(Expression::Traversal(Box::new(self.parse_traversal(p)?)));
                }
                Rule::id_traversal => {
                    expressions.push(Expression::Traversal(Box::new(self.parse_traversal(p)?)));
                }
                Rule::evaluates_to_bool => {
                    expressions.push(self.parse_boolean_expression(p)?);
                }
                _ => unreachable!(),
            }
        }
        Ok(expressions)
    }

    fn parse_boolean_expression(&self, pair: Pair<Rule>) -> Result<Expression, ParserError> {
        let expression = pair.into_inner().next().unwrap();
        match expression.as_rule() {
            Rule::and => Ok(Expression::And(
                self.parse_expression_vec(expression.into_inner())?,
            )),
            Rule::or => Ok(Expression::Or(
                self.parse_expression_vec(expression.into_inner())?,
            )),
            Rule::boolean => Ok(Expression::BooleanLiteral(expression.as_str() == "true")),
            Rule::exists => Ok(Expression::Exists(Box::new(
                self.parse_anon_traversal(expression.into_inner().next().unwrap())?,
            ))),
            _ => unreachable!(),
        }
    }

    fn parse_expression(&self, p: Pair<Rule>) -> Result<Expression, ParserError> {
        let pair = p
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::from("Empty expression"))?;

        match pair.as_rule() {
            Rule::traversal => Ok(Expression::Traversal(Box::new(self.parse_traversal(pair)?))),
            Rule::id_traversal => Ok(Expression::Traversal(Box::new(self.parse_traversal(pair)?))),
            Rule::anonymous_traversal => Ok(Expression::Traversal(Box::new(
                self.parse_anon_traversal(pair)?,
            ))),
            Rule::identifier => Ok(Expression::Identifier(pair.as_str().to_string())),
            Rule::string_literal => Ok(Expression::StringLiteral(self.parse_string_literal(pair)?)),
            Rule::exists => {
                let traversal = pair
                    .into_inner()
                    .next()
                    .ok_or_else(|| ParserError::from("Missing exists traversal"))?;
                Ok(Expression::Exists(Box::new(match traversal.as_rule() {
                    Rule::traversal => self.parse_traversal(traversal)?,
                    Rule::id_traversal => self.parse_traversal(traversal)?,
                    _ => unreachable!(),
                })))
            }
            Rule::integer => pair
                .as_str()
                .parse()
                .map(Expression::IntegerLiteral)
                .map_err(|_| ParserError::from("Invalid integer literal")),
            Rule::float => pair
                .as_str()
                .parse()
                .map(Expression::FloatLiteral)
                .map_err(|_| ParserError::from("Invalid float literal")),
            Rule::boolean => Ok(Expression::BooleanLiteral(pair.as_str() == "true")),
            Rule::evaluates_to_bool => Ok(self.parse_boolean_expression(pair)?),
            Rule::AddN => Ok(Expression::AddNode(self.parse_add_vertex(pair)?)),
            Rule::AddV => Ok(Expression::AddVector(self.parse_add_vector(pair)?)),
            Rule::BatchAddV => Ok(Expression::BatchAddVector(self.parse_batch_add_vector(pair)?)),
            Rule::AddE => Ok(Expression::AddEdge(self.parse_add_edge(pair, false)?)),
            Rule::search_vector => Ok(Expression::SearchVector(self.parse_search_vector(pair)?)),
            Rule::none => Ok(Expression::None),
            _ => Err(ParserError::from(format!(
                "Unexpected expression type: {:?}",
                pair.as_rule()
            ))),
        }
    }

    fn parse_string_literal(&self, pair: Pair<Rule>) -> Result<String, ParserError> {
        let inner = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::from("Empty string literal"))?;

        let mut literal = inner.as_str().to_string();
        literal.retain(|c| c != '"');
        Ok(literal)
    }

    fn parse_traversal(&self, pair: Pair<Rule>) -> Result<Traversal, ParserError> {
        let mut pairs = pair.into_inner();
        let start = self.parse_start_node(pairs.next().unwrap())?;
        let steps = pairs
            .map(|p| self.parse_step(p))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Traversal { start, steps })
    }

    fn parse_anon_traversal(&self, pair: Pair<Rule>) -> Result<Traversal, ParserError> {
        let pairs = pair.into_inner();
        let start = StartNode::Anonymous;
        let steps = pairs
            .map(|p| self.parse_step(p))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Traversal { start, steps })
    }

    fn parse_start_node(&self, pair: Pair<Rule>) -> Result<StartNode, ParserError> {
        match pair.as_rule() {
            Rule::start_vertex => {
                let pairs = pair.into_inner();
                let mut types = None;
                let mut ids = None;
                for p in pairs {
                    match p.as_rule() {
                        Rule::type_args => {
                            types = Some(
                                p.into_inner()
                                    .map(|t| t.as_str().to_string())
                                    .collect::<Vec<_>>(),
                            );
                        }
                        Rule::id_args => {
                            ids = Some(
                                p.into_inner()
                                    .map(|id| id.as_str().to_string())
                                    .collect::<Vec<_>>(),
                            );
                        }
                        _ => unreachable!(),
                    }
                }
                Ok(StartNode::Node { types, ids })
            }
            Rule::start_edge => {
                let pairs = pair.into_inner();
                let mut types = None;
                let mut ids = None;
                for p in pairs {
                    match p.as_rule() {
                        Rule::type_args => {
                            types = Some(
                                p.into_inner()
                                    .map(|t| t.as_str().to_string())
                                    .collect::<Vec<_>>(),
                            );
                        }
                        Rule::id_args => {
                            ids = Some(
                                p.into_inner()
                                    .map(|id| id.as_str().to_string())
                                    .collect::<Vec<_>>(),
                            );
                        }
                        _ => unreachable!(),
                    }
                }
                Ok(StartNode::Edge { types, ids })
            }
            Rule::identifier => Ok(StartNode::Variable(pair.as_str().to_string())),
            _ => Ok(StartNode::Anonymous),
        }
    }

    fn parse_step(&self, pair: Pair<Rule>) -> Result<Step, ParserError> {
        let inner = pair.clone().into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::graph_step => Ok(Step::Node(self.parse_graph_step(inner))),
            Rule::object_step => Ok(Step::Object(self.parse_object_step(inner)?)),
            Rule::closure_step => Ok(Step::Closure(self.parse_closure(inner)?)),
            Rule::where_step => Ok(Step::Where(Box::new(self.parse_expression(inner)?))),
            Rule::range_step => Ok(Step::Range(self.parse_range(pair)?)),

            Rule::bool_operations => Ok(Step::BooleanOperation(self.parse_bool_operation(inner)?)),
            Rule::count => Ok(Step::Count),
            Rule::ID => Ok(Step::Object(Object {
                fields: vec![("id".to_string(), FieldValue::Empty)],
                should_spread: false,
            })),
            Rule::update => Ok(Step::Update(self.parse_update(inner)?)),
            Rule::exclude_field => Ok(Step::Exclude(self.parse_exclude(inner)?)),
            Rule::AddE => Ok(Step::AddEdge(self.parse_add_edge(inner, true)?)),
            _ => Err(ParserError::from(format!(
                "Unexpected step type: {:?}",
                inner.as_rule()
            ))),
        }
    }

    fn parse_range(&self, pair: Pair<Rule>) -> Result<(Expression, Expression), ParserError> {
        let mut inner = pair.into_inner().next().unwrap().into_inner();
        // println!("inner: {:?}", inner);
        let start = match self.parse_expression(inner.next().unwrap()) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        let end = match self.parse_expression(inner.next().unwrap()) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };

        Ok((start, end))
    }

    fn parse_graph_step(&self, pair: Pair<Rule>) -> GraphStep {
        let rule_str = pair.as_str();
        let types = pair
            .into_inner()
            .next()
            .map(|p| p.into_inner().map(|t| t.as_str().to_string()).collect());

        match rule_str {
            s if s.starts_with("OutE") => GraphStep::OutE(types),
            s if s.starts_with("InE") => GraphStep::InE(types),
            s if s.starts_with("BothE") => GraphStep::BothE(types),
            s if s.starts_with("OutN") => GraphStep::OutN,
            s if s.starts_with("InN") => GraphStep::InN,
            s if s.starts_with("BothN") => GraphStep::BothN,
            s if s.starts_with("Out") => GraphStep::Out(types),
            s if s.starts_with("In") => GraphStep::In(types),
            s if s.starts_with("Both") => GraphStep::Both(types),
            // s if s.starts_with("Range") => GraphStep::Range(),
            _ => unreachable!(),
        }
    }

    fn parse_bool_operation(&self, pair: Pair<Rule>) -> Result<BooleanOp, ParserError> {
        let inner = pair.into_inner().next().unwrap();
        let expr = match inner.as_rule() {
            Rule::GT => BooleanOp::GreaterThan(Box::new(
                self.parse_expression(inner.into_inner().next().unwrap())?,
            )),
            Rule::GTE => BooleanOp::GreaterThanOrEqual(Box::new(
                self.parse_expression(inner.into_inner().next().unwrap())?,
            )),
            Rule::LT => BooleanOp::LessThan(Box::new(
                self.parse_expression(inner.into_inner().next().unwrap())?,
            )),
            Rule::LTE => BooleanOp::LessThanOrEqual(Box::new(
                self.parse_expression(inner.into_inner().next().unwrap())?,
            )),
            Rule::EQ => BooleanOp::Equal(Box::new(
                self.parse_expression(inner.into_inner().next().unwrap())?,
            )),
            Rule::NEQ => BooleanOp::NotEqual(Box::new(
                self.parse_expression(inner.into_inner().next().unwrap())?,
            )),
            _ => return Err(ParserError::from("Invalid boolean operation")),
        };
        Ok(expr)
    }

    fn parse_field_additions(&self, pair: Pair<Rule>) -> Result<Vec<FieldAddition>, ParserError> {
        pair.into_inner()
            .map(|p| self.parse_new_field_pair(p))
            .collect()
    }

    fn parse_new_field_pair(&self, pair: Pair<Rule>) -> Result<FieldAddition, ParserError> {
        let print_pair = pair.clone();
        let mut pairs = pair.into_inner();
        let name = pairs.next().unwrap().as_str().to_string();
        let value_pair = pairs.next().unwrap();

        let value: FieldValue = match value_pair.as_rule() {
            Rule::evaluates_to_anything => {
                FieldValue::Expression(self.parse_expression(value_pair)?)
            }
            Rule::anonymous_traversal => {
                FieldValue::Traversal(Box::new(self.parse_traversal(value_pair)?))
            }
            Rule::object_step => FieldValue::Fields(self.parse_field_additions(value_pair)?),
            Rule::string_literal => {
                FieldValue::Literal(Value::String(self.parse_string_literal(value_pair)?))
            }
            Rule::integer => FieldValue::Literal(Value::Integer(
                value_pair
                    .as_str()
                    .parse()
                    .map_err(|_| ParserError::from("Invalid integer literal"))?,
            )),
            Rule::float => FieldValue::Literal(Value::Float(
                value_pair
                    .as_str()
                    .parse()
                    .map_err(|_| ParserError::from("Invalid float literal"))?,
            )),
            Rule::boolean => FieldValue::Literal(Value::Boolean(value_pair.as_str() == "true")),
            Rule::none => FieldValue::Empty,
            Rule::mapping_field => FieldValue::Fields(self.parse_field_additions(value_pair)?),
            _ => {
                return Err(ParserError::from(format!(
                    "Unexpected field pair type: {:?} \n {:?} \n\n {:?}",
                    value_pair.as_rule(),
                    value_pair,
                    print_pair
                )))
            }
        };

        Ok(FieldAddition { name, value })
    }

    fn parse_new_field_value(&self, pair: Pair<Rule>) -> Result<FieldValue, ParserError> {
        let print_pair = pair.clone();
        let value_pair = pair.into_inner().next().unwrap();
        let value: FieldValue = match value_pair.as_rule() {
            Rule::evaluates_to_anything => {
                FieldValue::Expression(self.parse_expression(value_pair)?)
            }
            Rule::anonymous_traversal => {
                FieldValue::Traversal(Box::new(self.parse_traversal(value_pair)?))
            }
            Rule::object_step => FieldValue::Fields(self.parse_field_additions(value_pair)?),
            Rule::string_literal => {
                FieldValue::Literal(Value::String(self.parse_string_literal(value_pair)?))
            }
            Rule::integer => FieldValue::Literal(Value::Integer(
                value_pair
                    .as_str()
                    .parse()
                    .map_err(|_| ParserError::from("Invalid integer literal"))?,
            )),
            Rule::float => FieldValue::Literal(Value::Float(
                value_pair
                    .as_str()
                    .parse()
                    .map_err(|_| ParserError::from("Invalid float literal"))?,
            )),
            Rule::boolean => FieldValue::Literal(Value::Boolean(value_pair.as_str() == "true")),
            Rule::none => FieldValue::Empty,
            Rule::mapping_field => FieldValue::Fields(self.parse_field_additions(value_pair)?),
            _ => {
                return Err(ParserError::from(format!(
                    "Unexpected field value type: {:?} \n {:?} \n\n {:?}",
                    value_pair.as_rule(),
                    value_pair,
                    print_pair
                )))
            }
        };

        Ok(value)
    }

    fn parse_update(&self, pair: Pair<Rule>) -> Result<Update, ParserError> {
        let fields = self.parse_field_additions(pair)?;
        Ok(Update { fields })
    }

    fn parse_object_step(&self, pair: Pair<Rule>) -> Result<Object, ParserError> {
        let mut fields = Vec::new();
        let mut should_spread = false;
        for p in pair.into_inner() {
            if p.as_rule() == Rule::spread_object {
                should_spread = true;
                continue;
            }
            let mut pairs = p.into_inner();
            let prop_key = pairs.next().unwrap().as_str().to_string();
            let field_addition = match pairs.next() {
                Some(p) => match p.as_rule() {
                    Rule::evaluates_to_anything => {
                        FieldValue::Expression(self.parse_expression(p)?)
                    }
                    Rule::anonymous_traversal => {
                        FieldValue::Traversal(Box::new(self.parse_traversal(p)?))
                    }
                    Rule::mapping_field => FieldValue::Fields(self.parse_field_additions(p)?),
                    Rule::object_step => FieldValue::Fields(
                        self.parse_object_step(p)?
                            .fields
                            .iter()
                            .map(|(k, v)| FieldAddition {
                                name: k.clone(),
                                value: v.clone(),
                            })
                            .collect(),
                    ),

                    _ => self.parse_new_field_value(p)?,
                },
                None if prop_key.len() > 0 => FieldValue::Literal(Value::String(prop_key.clone())),
                None => FieldValue::Empty,
            };
            fields.push((prop_key, field_addition));
        }
        Ok(Object {
            fields,
            should_spread,
        })
    }

    fn parse_closure(&self, pair: Pair<Rule>) -> Result<Closure, ParserError> {
        let mut pairs = pair.clone().into_inner();
        let identifier = pairs.next().unwrap().as_str().to_string();
        let object = self.parse_object_step(pairs.next().unwrap())?;
        Ok(Closure { identifier, object })
    }

    fn parse_exclude(&self, pair: Pair<Rule>) -> Result<Exclude, ParserError> {
        let mut fields = Vec::new();
        for p in pair.into_inner() {
            fields.push(p.as_str().to_string());
        }
        Ok(Exclude { fields })
    }
}

// Tests module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_node_schema() {
        let input = r#"
        N::User {
            Name: String,
            Age: Integer
        }
        "#;

        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.node_schemas.len(), 1);
        let schema = &result.node_schemas[0];
        assert_eq!(schema.name, "User");
        assert_eq!(schema.fields.len(), 2);
    }

    #[test]
    fn test_parse_edge_schema() {
        let input = r#"
        
        E::Follows {
            From: User,
            To: User,
            Properties: {
                Since: Float
            }
        }
        "#;

        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.edge_schemas.len(), 1);
        let schema = &result.edge_schemas[0];
        assert_eq!(schema.name, "Follows");
        assert_eq!(schema.from, "User");
        assert_eq!(schema.to, "User");
        assert!(schema.properties.is_some());
        let properties = schema.properties.as_ref().unwrap();
        assert_eq!(properties.len(), 1);
        assert_eq!(properties[0].name, "Since");
        matches!(properties[0].field_type, FieldType::Float);
    }

    #[test]
    fn test_parse_edge_schema_no_props() {
        let input = r#"
        
        E::Follows {
            From: User,
            To: User,
            Properties: {
            }
        }
        "#;

        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.edge_schemas.len(), 1);
        let schema = &result.edge_schemas[0];
        assert_eq!(schema.name, "Follows");
        assert_eq!(schema.from, "User");
        assert_eq!(schema.to, "User");
        assert!(schema.properties.is_some());
        let properties = schema.properties.as_ref().unwrap();
        assert_eq!(properties.len(), 0);
    }

    #[test]
    fn test_parse_query() {
        let input = r#"
        QUERY FindUser(userName : String) => 
            user <- N<User>
            RETURN user
        "#;

        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.queries.len(), 1);
        let query = &result.queries[0];
        assert_eq!(query.name, "FindUser");
        assert_eq!(query.parameters.len(), 1);
        assert_eq!(query.parameters[0].name, "userName");
        assert_eq!(query.statements.len(), 1);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_query_with_parameters() {
        let input = r#"
        QUERY fetchUsers(name: String, age: Integer) =>
            user <- N<USER>("123")
            nameField <- user::{Name}
            ageField <- user::{Age}
            RETURN nameField, ageField
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.queries.len(), 1);
        let query = &result.queries[0];
        assert_eq!(query.name, "fetchUsers");
        assert_eq!(query.parameters.len(), 2);
        assert_eq!(query.parameters[0].name, "name");
        assert_eq!(query.parameters[0].param_type, FieldType::String);
        assert_eq!(query.parameters[1].name, "age");
        assert_eq!(query.parameters[1].param_type, FieldType::Integer);
        assert_eq!(query.statements.len(), 3);
        assert_eq!(query.return_values.len(), 2);
    }

    #[test]
    fn test_node_definition() {
        let input = r#"
        N::USER {
            ID: String,
            Name: String,
            Age: Integer
        }
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.node_schemas.len(), 1);
        let schema = &result.node_schemas[0];
        assert_eq!(schema.name, "USER");
        assert_eq!(schema.fields.len(), 3);
    }

    #[test]
    fn test_edge_with_properties() {
        let input = r#"
        E::FRIENDSHIP {
            From: USER,
            To: USER,
            Properties: {
                Since: String,
                Strength: Integer
            }
        }
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.edge_schemas.len(), 1);
        let schema = &result.edge_schemas[0];
        assert_eq!(schema.name, "FRIENDSHIP");
        assert_eq!(schema.from, "USER");
        assert_eq!(schema.to, "USER");
        let props = schema.properties.as_ref().unwrap();
        assert_eq!(props.len(), 2);
    }

    #[test]
    fn test_multiple_schemas() {
        let input = r#"
        N::USER {
            ID: String,
            Name: String,
            Email: String
        }
        N::POST {
            ID: String,
            Content: String
        }
        E::LIKES {
            From: USER,
            To: POST,
            Properties: {
                Timestamp: String
            }
        }
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.node_schemas.len(), 2);
        assert_eq!(result.edge_schemas.len(), 1);
    }

    /// THESE FAIL
    ///
    ///
    ///

    #[test]
    fn test_logical_operations() {
        let input = r#"
    QUERY logicalOps(id : String) =>
        user <- N<USER>(id)
        condition <- user::{name}::EQ("Alice")
        condition2 <- user::{age}::GT(20)
        RETURN condition
    "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.name, "logicalOps");
        assert_eq!(query.statements.len(), 3);
    }

    #[test]
    fn test_anonymous_traversal() {
        let input = r#"
    QUERY anonymousTraversal() =>
        result <- N::OutE<FRIENDSHIP>::InN::{Age}
        RETURN result
    "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.name, "anonymousTraversal");
        assert_eq!(query.statements.len(), 1);
    }

    #[test]
    fn test_edge_traversal() {
        let input = r#"
    QUERY getEdgeInfo() =>
        edge <- E<FRIENDSHIP>("999")
        fromUser <- edge::OutE
        toUser <- edge::OutN
        RETURN fromUser, toUser

    "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 3);
        assert_eq!(query.return_values.len(), 2);
    }

    #[test]
    fn test_exists_query() {
        let input = r#"
        QUERY userExists(id : String) =>
            user <- N<User>(id)
            result <- EXISTS(user::OutE::InN<User>)
            RETURN result
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.queries.len(), 1);
        let query = &result.queries[0];
        assert_eq!(query.name, "userExists");
        assert_eq!(query.parameters.len(), 1);
        assert_eq!(query.statements.len(), 2);
    }

    #[test]
    fn test_multiple_return_values() {
        let input = r#"
    QUERY returnMultipleValues() =>
        user <- N<USER>("999")
        name <- user::{Name}
        age <- user::{Age}
        RETURN name, age
    "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 3);
        assert_eq!(query.return_values.len(), 2);
    }

    #[test]
    fn test_add_fields() {
        let input = r#"
    QUERY enrichUserData() =>
        user <- N<USER>("123")
        enriched <- user::{Name: "name", Follows: _::Out<Follows>::{Age}}
        RETURN enriched
    "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
    }

    #[test]
    fn test_query_with_count() {
        let input = r#"
    QUERY analyzeNetwork() =>
        user <- N<USER>("999")
        friends <- user::Out<FRIENDSHIP>::InN::WHERE(_::Out::COUNT::GT(0))
        friendCount <- activeFriends::COUNT
        RETURN friendCount
    "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 3);
    }

    #[test]
    fn test_add_vertex_query() {
        let input = r#"
    QUERY analyzeNetwork() =>
        user <- AddN<User>({Name: "Alice"})
        RETURN user
    "#;
        let result = match HelixParser::parse_source(input) {
            Ok(result) => result,
            Err(e) => {
                println!("{:?}", e);
                panic!();
            }
        };
        let query = &result.queries[0];
        // println!("{:?}", query);
        assert_eq!(query.statements.len(), 1);
    }

    #[test]
    fn test_add_edge_query() {
        let input = r#"
    QUERY analyzeNetwork() =>
        edge <- AddE<Rating>({Rating: 5})::To("123")::From("456")
        edge <- AddE<Rating>({Rating: 5, Date: "2025-01-01"})::To("123")::From("456")
        RETURN edge
    "#;

        let result = match HelixParser::parse_source(input) {
            Ok(result) => result,
            Err(e) => {
                println!("{:?}", e);
                panic!();
            }
        };
        let query = &result.queries[0];
        // println!("{:?}", query);
        assert_eq!(query.statements.len(), 2);
    }

    #[test]
    fn test_adding_with_identifiers() {
        let input = r#"
    QUERY addUsers() =>
        user1 <- AddN<User>({Name: "Alice", Age: 30})
        user2 <- AddN<User>({Name: "Bob", Age: 25})
        AddE<Follows>({Since: "1.0"})::From(user1)::To(user2)
        RETURN user1, user2
    "#;

        let result = match HelixParser::parse_source(input) {
            Ok(result) => result,
            Err(e) => {
                println!("{:?}", e);
                panic!();
            }
        };
        // println!("{:?}", result);
        let query = &result.queries[0];
        // println!("{:?}", query);
        assert_eq!(query.statements.len(), 3);
    }

    #[test]
    fn test_where_with_props() {
        let input = r#"
    QUERY getFollows() =>
        user <- N<User>::WHERE(_::{Age}::GT(2))
        user <- N<User>::WHERE(_::GT(2))
        RETURN user, follows
        "#;

        let result = match HelixParser::parse_source(input) {
            Ok(result) => result,
            Err(_e) => {
                panic!();
            }
        };
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
    }

    #[test]
    fn test_drop_operation() {
        let input = r#"
        QUERY deleteUser(id: String) =>
            user <- N<USER>(id)
            DROP user
            DROP user::OutE
            DROP N::OutE
            RETURN user
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.name, "deleteUser");
        assert_eq!(query.parameters.len(), 1);
        assert_eq!(query.statements.len(), 4);
    }

    #[test]
    fn test_update_operation() {
        let input = r#"
        QUERY updateUser(id: String) =>
            user <- N<USER>(id)
            x <- user::UPDATE({Name: "NewName"})
            l <- user::UPDATE({Name: "NewName", Age: 30})
            RETURN user
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.name, "updateUser");
        assert_eq!(query.parameters.len(), 1);
        assert_eq!(query.statements.len(), 3);
    }

    #[test]
    fn test_complex_traversal_combinations() {
        let input = r#"
        QUERY complexTraversal() =>
            result1 <- N<User>::OutE<Follows>::InN<User>::{name}
            result2 <- N::WHERE(AND(
                _::{age}::GT(20),
                OR(_::{name}::EQ("Alice"), _::{name}::EQ("Bob"))
            ))
            result3 <- N<User>::{
                friends: _::Out<Follows>::InN::{name},
                avgFriendAge: _::Out<Follows>::InN::{age}::GT(25)
            }
            RETURN result1, result2, result3
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.name, "complexTraversal");
        assert_eq!(query.statements.len(), 3);
        assert_eq!(query.return_values.len(), 3);
    }

    #[test]
    fn test_nested_property_operations() {
        let input = r#"
        QUERY nestedProps() =>
            user <- N<User>("123")
            // Test nested property operations
            result <- user::{
                basic: {
                    name: _::{name},
                    age: _::{age}
                },
                social: {
                    friends: _::Out<Follows>::COUNT,
                    groups: _::Out<BelongsTo>::InN<Group>::{name}
                }
            }
            RETURN result
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
    }

    #[test]
    fn test_complex_edge_operations() {
        let input = r#"
        QUERY edgeOperations() =>
            edge1 <- AddE<Follows>({since: "2024-01-01", weight: 0.8})::From("user1")::To("user2")
            edge2 <- E<Follows>::WHERE(_::{weight}::GT(0.5))
            edge3 <- edge2::UPDATE({weight: 1.0, updated: "2024-03-01"})
            RETURN edge1, edge2, edge3
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 3);
        assert_eq!(query.return_values.len(), 3);
    }

    #[test]
    fn test_mixed_type_operations() {
        let input = r#"
        QUERY mixedTypes() =>
            v1 <- AddN<User>({
                name: "Alice",
                age: 25,
                active: true,
                score: 4.5
            })
            result <- N<User>::WHERE(OR(
                _::{age}::GT(20),
                _::{score}::LT(5.0)
            ))
            RETURN v1, result
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
        assert_eq!(query.return_values.len(), 2);
    }

    #[test]
    fn test_error_cases() {
        // Test missing return statement
        let missing_return = r#"
        QUERY noReturn() =>
            result <- N<User>()
        "#;

        let result = HelixParser::parse_source(missing_return);
        assert!(result.is_err());

        // Test invalid property access
        let invalid_props = r#"
        QUERY invalidProps() =>
            result <- N<User>::{}
            RETURN result
        "#;

        let result = HelixParser::parse_source(invalid_props);
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_schema_definitions() {
        let input = r#"
        N::ComplexUser {
            ID: String,
            Name: String,
            Age: Integer,
            Score: Float,
            Active: Boolean
        }
        E::ComplexRelation {
            From: ComplexUser,
            To: ComplexUser,
            Properties: {
                StartDate: String,
                EndDate: String,
                Weight: Float,
                Valid: Boolean,
                Count: Integer
            }
        }
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        assert_eq!(result.node_schemas.len(), 1);
        assert_eq!(result.edge_schemas.len(), 1);

        let node = &result.node_schemas[0];
        assert_eq!(node.fields.len(), 5);

        let edge = &result.edge_schemas[0];
        let props = edge.properties.as_ref().unwrap();
        assert_eq!(props.len(), 5);
    }

    #[test]
    fn test_query_chaining() {
        let input = r#"
        QUERY chainedOperations() =>
            result <- N<User>("123")
                ::OutE<Follows>
                ::InN<User>
                ::{name}
                ::EQ("Alice")
            filtered <- N<User>::WHERE(
                _::Out<Follows>
                    ::InN<User>
                    ::{age}
                    ::GT(25)
            )
            updated <- filtered
                ::UPDATE({status: "active"})
            has_updated <- updated::{status}
                ::EQ("active")
            RETURN result, filtered, updated, has_updated
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 4);
        assert_eq!(query.return_values.len(), 4);
    }

    #[test]
    fn test_property_assignments() {
        let input = r#"
        QUERY testProperties(age: Integer) =>
            user <- AddN<User>({
                name: "Alice",
                age: age
            })
            RETURN user
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.parameters.len(), 1);
    }

    #[test]
    fn test_map_operation() {
        let input = r#"
        QUERY mapOperation() =>
            user <- N<User>("123")
            mapped <- user::{name: "name", age: "age"}
            RETURN mapped
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_map_in_return() {
        let input = r#"
        QUERY mapInReturn() =>
            user <- N<User>("123")
            RETURN user::{
                name, 
                age
            }
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 1);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_complex_object_operations() {
        let input = r#"
        QUERY complexObjects() =>
            user <- N<User>("123")
            result <- user::{
                basic: {
                    name,
                    age
                },
                friends: _::Out<Follows>::InN::{
                    name,
                    mutualFriends: _::Out<Follows>::COUNT
                }
            }
            RETURN result
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_exclude_fields() {
        let input = r#"
        QUERY excludeFields() =>
            user <- N<User>("123")
            filtered <- user::!{password, secretKey}
            RETURN filtered
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_spread_operator() {
        let input = r#"
        QUERY spreadFields() =>
            user <- N<User>("123")
            result <- user::{
                newField: "value",
                ..
            }
            RETURN result
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_complex_update_operations() {
        let input = r#"
        QUERY updateUser() =>
            user <- N<User>("123")
            updated <- user::UPDATE({
                name: "New Name",
                age: 30,
                lastUpdated: "2024-03-01",
                friendCount: _::Out<Follows>::COUNT
            })
            RETURN updated
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 2);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_nested_traversals() {
        let input = r#"
        QUERY nestedTraversals() =>
            start <- N<User>("123")
            result <- start::Out<Follows>::InN<User>::Out<Likes>::InN<Post>::{title}
            filtered <- result::WHERE(_::{likes}::GT(10))
            RETURN filtered
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 3);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_combined_operations() {
        let input = r#"
        QUERY combinedOps() =>
            // Test combination of different operations
            user <- N<User>("123")
            friends <- user::Out<Follows>::InN<User>
            active <- friends::WHERE(_::{active}::EQ(true))
            result <- active::{
                name,
                posts: _::Out<Created>::InN<Post>::!{deleted}::{
                    title: title,
                    likes: _::In<Likes>::COUNT
                }
            }
            RETURN result
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 4);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_closure() {
        let input = r#"
        QUERY multipleLayers() =>
            result <- N<User>::|user|{
                posts: _::Out<Created>::{
                    user_id: user::ID
                }
            }
            RETURN result
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        // println!("\n\nresult: {:?}\n\n", result);
        let query = &result.queries[0];
        assert_eq!(query.statements.len(), 1);
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_complex_return_traversal() {
        let input = r#"
        QUERY returnTraversal() =>
            RETURN N<User>::|user|{
                posts: _::Out<Created>::{
                    user_id: user::ID
                }
            }::!{createdAt, lastUpdated}::{username: name, ..}
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_array_as_param_type() {
        let input = r#"
        QUERY trWithArrayParam(ids: [String], names:[String], ages: [Integer], createdAt: String) => 
            AddN<User>({Name: "test"})
            RETURN "SUCCESS"
        "#;

        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.return_values.len(), 1);

        assert!(query.parameters.iter().any(|param| match param.param_type {
            FieldType::String => true,
            _ => false,
        }));
        assert!(query.parameters.iter().any(|param| match param.param_type {
            FieldType::Array(ref field) => match &**field {
                FieldType::String =>
                    if param.name == "names" || param.name == "ids" {
                        true
                    } else {
                        false
                    },
                _ => false,
            },
            _ => false,
        }));
        assert!(query.parameters.iter().any(|param| match param.param_type {
            FieldType::Array(ref field) => match &**field {
                FieldType::Integer =>
                    if param.name == "ages" {
                        true
                    } else {
                        false
                    },
                _ => false,
            },
            _ => false,
        }))
    }

    #[test]
    fn test_schema_obj_as_param_type() {
        let input = r#"
        N::User {
            Name: String
        }

        QUERY trWithArrayParam(user: User) => 
            AddN<User>({Name: "test"})
            RETURN "SUCCESS"
        "#;

        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.return_values.len(), 1);

        // println!("{:?}", query.parameters);
        let mut param_type = "";
        assert!(
            query.parameters.iter().any(|param| match param.param_type {
                FieldType::Identifier(ref id) => match id.as_str() {
                    "User" => true,
                    _ => {
                        param_type = id;
                        false
                    }
                },
                _ => false,
            }),
            "Param of type {} was not found",
            param_type
        );
    }

    #[test]
    fn test_add_vector() {
        let input = r#"
        V::User 

        QUERY addVector(vector: [Float]) =>
            RETURN AddV<User>(vector)
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_bulk_insert() {
        let input = r#"
        QUERY bulkInsert(vectors: [[Float]]) =>
            vectors::AddV<User>
            RETURN "SUCCESS"
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.return_values.len(), 1);
    }

    #[test]
    fn test_search_vector() {
        let input = r#"
        V::User 

        QUERY searchVector(vector: [Float]) =>
            RETURN SearchV<User>(vector)
        "#;
        let result = HelixParser::parse_source(input).unwrap();
        let query = &result.queries[0];
        assert_eq!(query.return_values.len(), 1);
    }
}
