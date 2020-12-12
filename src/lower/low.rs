use std::sync::Arc;

use crate::node::{HIndex, LNode, Value, VPath};
use crate::query::{MScope, QScope};

pub fn low(scope: QScope, path: &VPath) -> crate::Result<Arc<LNode>> {
	scope.ctx.low.scope(scope, path.clone(), |scope| {
		unimplemented!()
	})
}

fn lower(scope: MScope, value: &Value, index: &HIndex) -> crate::Result<Option<LNode>> {
	unimplemented!()
}
