use super::super::*;

#[test]
fn parse_transaction_statement_success() {
    let source = source("fn t(): null { transaction source, target { return } }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let transaction_statement = body_node.first_child().unwrap();
    let transaction_node = syntax.node(transaction_statement).unwrap();

    assert_eq!(
        transaction_node.kind(),
        SyntaxNodeKind::TransactionStatement
    );
    assert_eq!(transaction_node.child_count(), 2);

    let target_list = transaction_node.child(0).unwrap();
    let target_list_node = syntax.node(target_list).unwrap();
    assert_eq!(
        target_list_node.kind(),
        SyntaxNodeKind::TransactionTargetList
    );
    assert_eq!(target_list_node.child_count(), 2);

    let block = transaction_node.child(1).unwrap();
    let block_node = syntax.node(block).unwrap();
    assert_eq!(block_node.kind(), SyntaxNodeKind::Block);
}

#[test]
fn parse_rollback_statement_success() {
    let source = source("fn r(): null { rollback }");

    let result = parse(&source);

    assert!(!result.has_errors());

    let syntax = result.graph().syntax();

    let root = syntax.root().unwrap();
    let function = syntax.node(root).unwrap().first_child().unwrap();
    let function_node = syntax.node(function).unwrap();

    let body = function_node.child(3).unwrap();
    let body_node = syntax.node(body).unwrap();

    let rollback_statement = body_node.first_child().unwrap();
    let rollback_node = syntax.node(rollback_statement).unwrap();

    assert_eq!(rollback_node.kind(), SyntaxNodeKind::RollbackStatement);
    assert_eq!(rollback_node.child_count(), 0);
}
