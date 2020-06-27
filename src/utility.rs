pub fn ceiling(value: usize, multiple: usize) -> usize {
	if value % multiple == 0 { return value; }
	let padding = multiple - (value % multiple);
	value + padding
}
