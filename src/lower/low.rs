use std::collections::HashMap;
use std::sync::Arc;

use crate::generate::Target;
use crate::inference::Types;
use crate::node::*;
use crate::query::{E, IScope, ISpan, ItemScope, QScope, QueryError, S};

use super::{lower, unit};

macro_rules! bail {
    ($value:expr) => {
    	$value.ok_or(QueryError::Failure)
    };
}

pub struct Scene<'a, 'b, 'c> {
	pub scope: IScope<'c, 'b, 'a>,
	pub locals: HashMap<LTarget, S<Size>>,
	pub targets: HashMap<Variable, LTarget>,
	pub types: &'a Types,
	pub node: &'a Value,
	pub target: Target,
}

impl<'a, 'b, 'c> Scene<'a, 'b, 'c> {
	pub fn new(scope: IScope<'c, 'b, 'a>, types: &'a Types,
			   node: &'a Value, target: Target) -> Self {
		let (locals, targets) = (HashMap::new(), HashMap::new());
		Scene { scope, locals, targets, types, node, target }
	}

	pub fn variable(&mut self, variable: Variable, size: S<Size>) -> LTarget {
		let target = self.local(size);
		assert!(!self.targets.contains_key(&variable));
		self.targets.insert(variable, target.clone());
		target
	}

	pub fn local(&mut self, size: S<Size>) -> LTarget {
		let target = LTarget(self.targets.len());
		self.locals.insert(target.clone(), size);
		target
	}
}

pub fn function(scope: QScope, path: &FLocal) -> crate::Result<Arc<LFunction>> {
	scope.ctx.lower.inherit(scope, path.clone(), |scope| {
		let function = crate::parse::local(scope, path)?;
		let types = &crate::inference::function(scope, path)?;
		let symbol = Symbol::Function(path.as_ref().clone());
		let target = crate::analysis::target(scope, &symbol)?;

		let scope = &mut ItemScope::new(scope, symbol);
		let target = target.ok_or_else(||
			E::error().message("unknown architecture")
				.note("specify the architecture with: @architecture")
				.label(function.name.span.label()).to(scope))?;

		let node = &function.value;
		let mut parameters = HashMap::new();
		let mut scene = Scene::new(scope, types, node, target);
		for name in function.signature.parameters.keys() {
			let variable = Variable::parameter(name.clone());
			let kind = &types.variables[&variable];
			let size = super::size(scene.scope, kind)?;
			let target = scene.variable(variable, size);
			parameters.insert(name.clone(), target);
		}

		let target = &function.signature.return_type;
		let kind = crate::inference::lift(scene.scope, target)?;
		let zero = super::size(scene.scope, &kind)?.node.zero();
		let unit = zero.then(|| unit(&mut scene, &node.root, false))
			.unwrap_or_else(|| lower(&mut scene, &node.root, false)
				.map(|node| S::new(LUnit::Return(Some(node)), target.span)))?;
		Ok(Arc::new(LFunction { unit, parameters, locals: scene.locals }))
	})
}

pub fn low(scope: QScope, path: &VPath) -> crate::Result<Arc<LValue>> {
	scope.ctx.low.inherit(scope, path.clone(), |scope| {
		let node = &crate::parse::value(scope, path)?;
		let types = &crate::inference::types(scope, path)?;
		let scope = &mut ItemScope::path(scope, path.clone());
		let mut scene = Scene::new(scope, types, node, Target::Host);

		let kind = bail!(types.nodes.get(&node.root))?;
		let size = super::size(scene.scope, kind)?;
		let value = match size.node.zero() {
			true => LValued::Block(unit(&mut scene, &node.root, false)?),
			false => LValued::Node(lower(&mut scene, &node.root, false)?),
		};

		Ok(Arc::new(LValue { value, locals: scene.locals }))
	})
}

pub fn place(scene: &mut Scene, index: &HIndex, looped: bool,
			 lift: bool) -> crate::Result<LPlace> {
	let span = scene.node[*index].span;
	let node = lower(scene, index, looped)?;
	if let LNode::Dereference(place) = node.node {
		return Ok(place);
	}

	if lift {
		let kind = bail!(scene.types.nodes.get(index))?;
		let size = super::size(scene.scope, kind)?;
		let local = scene.local(size);

		let target = Box::new(S::new(LNode::Target(local), span));
		let unit = S::new(LUnit::Set(LPlace(target.clone()), node), span);
		let block = LNode::Block(Box::new([unit]), target);
		Ok(LPlace(Box::new(S::new(block, span))))
	} else {
		let error = E::error().message("temporary value has no location");
		let error = error.note("move this value into a local variable");
		error.label(span.label()).result(scene.scope)
	}
}

pub fn offset(scene: &mut Scene, place: LPlace,
			  offset: usize, span: ISpan) -> LPlace {
	let offset = LNode::Integral(offset as i128);
	offset_node(scene, place, offset, span)
}

pub fn offset_node(scene: &mut Scene, LPlace(node): LPlace,
				   offset: LNode, span: ISpan) -> LPlace {
	let width = scene.target.pointer;
	let offset = Box::new(S::new(offset, span));
	let binary = LNode::Binary(LBinary::Add, width, node, offset);
	LPlace(Box::new(S::new(binary, span)))
}

pub fn invalid_cast<T>(scene: &mut Scene, origin: &RType, kind: &RType,
					   span: ISpan, other: ISpan) -> crate::Result<T> {
	let error = E::error().message("invalid cast");
	let error = error.label(span.other().message(&origin));
	error.label(other.label().message(kind)).result(scene.scope)
}

pub fn invalid_unary<T>(scene: &mut Scene, kind: &RType,
						span: ISpan) -> crate::Result<T> {
	let error = E::error().message("invalid unary operation");
	error.label(span.label().message(kind)).result(scene.scope)
}

pub fn field<'a>(scene: &mut Scene, fields: &'a HFields, name: &Identifier,
				 span: ISpan) -> crate::Result<&'a (ISpan, HIndex)> {
	fields.get(name).ok_or_else(|| {
		let message = format!("missing field: {}", name);
		E::error().message(message).label(span.label()).to(scene.scope)
	})
}

pub fn pointer(scene: &mut Scene, target: &Option<Target>,
			   span: ISpan) -> crate::Result<Width> {
	match target {
		None => E::error().message("unknown architecture")
			.label(span.label()).result(scene.scope),
		Some(target) => Ok(target.pointer),
	}
}

pub fn functional(scene: &mut Scene, path: &HPath, index: FIndex,
				  span: ISpan) -> crate::Result<LReceiver> {
	let path = S::new(FPath(path.path(), index), span);
	let symbol = &Symbol::Function(path.node.clone());
	let scope = &mut scene.scope.span(span);

	// Unknown target errors are handled on receiver lowering.
	let target = bail!(crate::analysis::target(scope, symbol)?)?;
	self::call_target(scene, target, span)?;
	Ok(LReceiver::Path(path))
}

pub fn method(scene: &mut Scene, base: &HIndex,
			  looped: bool) -> crate::Result<LReceiver> {
	let node = lower(scene, base, looped)?;
	let kind = bail!(scene.types.nodes.get(base))?;
	if let RType::Function(function) = &kind.node {
		let target = function.target.ok_or_else(||
			E::error().message("unknown architecture")
				.label(node.span.label()).to(scene.scope))?;

		self::call_target(scene, target, node.span)?;
		let convention = function.convention.clone();
		Ok(LReceiver::Method(convention, Box::new(node)))
	} else { panic!("type: {}, not function", kind.node) }
}

pub fn call_target(scene: &mut Scene, target: Target,
				   span: ISpan) -> crate::Result<()> {
	Ok(if scene.target != Target::Host && scene.target != target {
		let message = "cannot call function of differing architecture";
		let note = format!("local architecture: {}", scene.target);
		let label = span.label().message(target.to_string());
		let error = E::error().message(message).note(note);
		return error.label(label).result(scene.scope);
	})
}

pub fn target(scene: &mut Scene, target: &Option<Target>,
			  span: ISpan, action: &str) -> crate::Result<()> {
	let target = target.ok_or_else(||
		E::error().message("unknown architecture")
			.label(span.label()).to(scene.scope))?;

	Ok(if scene.target != target {
		let message = format!("cannot {} of differing architecture", action);
		let note = format!("local architecture: {}", scene.target);
		let label = span.label().message(target.to_string());
		let error = E::error().message(message).note(note);
		return error.label(label).result(scene.scope);
	})
}
