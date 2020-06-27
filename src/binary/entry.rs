use crate::context::Context;
use crate::generate::Section;
use crate::node::{Item, Symbol};
use crate::node::address::{Address, SymbolSize};

#[derive(Debug)]
pub struct Entry {
	pub load: Address,
	pub address: Address,
	pub size: SymbolSize,
	pub entity: Entity,
}

#[derive(Debug)]
pub enum Entity {
	Function(Section),
	Variable(Option<Vec<u8>>),
}

#[derive(Debug)]
pub struct Segment {
	pub address: Address,
	pub kind: SegmentKind,
}

#[derive(Debug)]
pub enum SegmentKind {
	Text(Vec<Vec<u8>>),
	Data(Vec<Vec<u8>>),
	Reserve(usize),
}

pub fn entries(context: &Context) -> crate::Result<Vec<Entry>> {
	let mut symbols = Vec::new();
	let items = context.items.read();
	items.iter().map(|item| match item {
		Item::Symbol(symbol @ Symbol::Function(path)) => {
			if !crate::node::present(context, None, path, None)? { return Ok(()); }
			let section = crate::generate::x64::lower(context, None, path, None)?;
			let address = crate::node::address::start(context, None, symbol, None)?;
			let load = crate::node::address::load(context, None, symbol, None)?;
			let size = crate::node::address::size(context, None, symbol, None)?;
			let entity = Entity::Function(section.as_ref().clone());
			Ok(symbols.push(Entry { load, address, size, entity }))
		}
		Item::Symbol(Symbol::Variable(_)) => unimplemented!(),
		Item::Symbol(Symbol::Module(_)) => Ok(()),
		Item::ModuleEnd => Ok(()),
	}).filter(Result::is_err).last().unwrap_or(Ok(()))?;
	Ok(symbols)
}

pub fn segments(mut entries: Vec<Entry>) -> Vec<Segment> {
	let mut segments = Vec::new();
	let segment: &mut Option<Segment> = &mut None;
	let mut last_address: Option<Address> = None;

	entries.sort_unstable_by_key(|entry| entry.load);
	for Entry { load: address, size, entity, .. } in entries {
		let kind = segment.as_mut().map(|segment| &mut segment.kind);
		let push = &mut |segment: &mut Option<Segment>, kind| {
			if let Some(segment) = segment.take() { segments.push(segment); }
			*segment = Some(Segment { address, kind });
		};

		match entity {
			Entity::Function(section) => match kind {
				Some(SegmentKind::Text(sections)) if last_address ==
					Some(address) => sections.push(section.bytes),
				_ => push(segment, SegmentKind::Text(vec![section.bytes]))
			},
			Entity::Variable(Some(other)) => match kind {
				Some(SegmentKind::Data(data)) if last_address ==
					Some(address) => data.push(other),
				_ => push(segment, SegmentKind::Data(vec![other])),
			},
			Entity::Variable(None) => match kind {
				Some(SegmentKind::Reserve(reserve)) if last_address ==
					Some(address) => *reserve += size,
				_ => push(segment, SegmentKind::Reserve(size)),
			}
		}

		last_address = Some(address + size);
	}

	segments.extend(segment.take());
	segments
}
