use super::parser::helix_parser::Source;

pub fn interpret_ast(ast: &Source) -> Result<(), Error> {
    let queries = ast.queries;
    let nodes = ast.node_schemas;
    let edges = ast.edge_schemas;

    for node in nodes {}
}
