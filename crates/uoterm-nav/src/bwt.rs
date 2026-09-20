//! The second packing step of the newer client packages. The gump art and
//! some other files are packed twice: a block sort first, then zlib. This
//! module undoes the block sort.
//!
//! The packed bytes have two layers. The outer layer is a move-to-front
//! code: each byte is a place in a list of all byte values, and the value
//! at that place then moves to the front. Its last byte is not part of the
//! data. The inner layer starts with how often each byte value is in the
//! output, as 256 numbers. After them, each value has one block of bytes, the
//! most frequent value first. A byte of a block says which value comes next,
//! as a place in a second move-to-front list.

const HEADER_BYTES: usize = 4;
const VALUES: usize = 256;
const COUNT_BYTES: usize = 4;
const COUNTS_BYTES: usize = VALUES * COUNT_BYTES;

/// Undoes the outer move-to-front code.
fn front_list_decode(packed: &[u8]) -> Vec<u8> {
    let mut list: [u8; VALUES] = std::array::from_fn(|i| i as u8);
    packed
        .iter()
        .map(|&place| {
            let place = usize::from(place);
            let value = list[place];
            list.copy_within(0..place, 1);
            list[0] = value;
            value
        })
        .collect()
}

/// The byte values that are in the output, the most frequent first. Of two
/// values with the same count the lower one is first.
fn by_frequency(counts: &[usize; VALUES]) -> Vec<u8> {
    let mut values: Vec<u8> = (0..=u8::MAX)
        .filter(|v| counts[usize::from(*v)] > 0)
        .collect();
    values.sort_by(|a, b| counts[usize::from(*b)].cmp(&counts[usize::from(*a)]));
    values
}

/// Unpacks one record. None for a record that is cut short or damaged.
pub(crate) fn unpack(packed: &[u8]) -> Option<Vec<u8>> {
    let coded = packed.get(HEADER_BYTES..packed.len().checked_sub(1)?)?;
    let inner = front_list_decode(coded);
    let blocks = inner.get(COUNTS_BYTES..)?;
    let mut counts = [0usize; VALUES];
    for (count, bytes) in counts.iter_mut().zip(inner.chunks_exact(COUNT_BYTES)) {
        *count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    }
    let len: usize = counts.iter().sum();
    if len > blocks.len() {
        return None;
    }
    // The block of each value: where its next byte is, and where it ends.
    // The first byte of a block says where the value starts in the list.
    let mut list: [u8; VALUES] = std::array::from_fn(|i| i as u8);
    let mut cursor = [0usize; VALUES];
    let mut end = [0usize; VALUES];
    let mut values_left = 0usize;
    let mut at = 0;
    for value in by_frequency(&counts) {
        let v = usize::from(value);
        list[usize::from(*blocks.get(at)?)] = value;
        cursor[v] = at + 1;
        at += counts[v];
        end[v] = at;
        values_left += 1;
    }
    let mut out = Vec::with_capacity(len);
    let mut value = list[0];
    while out.len() < len {
        out.push(value);
        let v = usize::from(value);
        if cursor[v] >= end[v] {
            // The block of this value is used up. It leaves the list.
            if values_left > 0 {
                values_left -= 1;
                list.copy_within(1..=values_left.min(VALUES - 1), 0);
                value = list[0];
            }
            continue;
        }
        let place = usize::from(*blocks.get(cursor[v])?);
        cursor[v] += 1;
        if place != 0 {
            list.copy_within(1..=place, 0);
            list[place] = value;
            value = list[0];
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The outer code of `plain`, for a test record.
    fn front_list_encode(plain: &[u8]) -> Vec<u8> {
        let mut list: Vec<u8> = (0..=u8::MAX).collect();
        plain
            .iter()
            .map(|value| {
                let place = list.iter().position(|v| v == value).unwrap();
                let value = list.remove(place);
                list.insert(0, value);
                place as u8
            })
            .collect()
    }

    fn record(counts: &[(u8, u32)], blocks: &[u8]) -> Vec<u8> {
        let mut inner = vec![0u8; COUNTS_BYTES];
        for (value, count) in counts {
            let at = usize::from(*value) * COUNT_BYTES;
            inner[at..at + COUNT_BYTES].copy_from_slice(&count.to_le_bytes());
        }
        inner.extend_from_slice(blocks);
        let mut packed = vec![0u8; HEADER_BYTES];
        packed.extend(front_list_encode(&inner));
        packed.push(0);
        packed
    }

    #[test]
    fn the_outer_code_moves_each_value_to_the_front() {
        let plain = [5u8, 5, 0, 7, 5];
        assert_eq!(front_list_decode(&front_list_encode(&plain)), plain);
    }

    #[test]
    fn one_value_three_times_comes_out_three_times() {
        let packed = record(&[(b'A', 3)], &[0, 0, 0]);
        assert_eq!(unpack(&packed).unwrap(), b"AAA");
    }

    #[test]
    fn two_values_take_turns_by_the_places_in_their_blocks() {
        // A is the more frequent, so its block is first. A starts at the
        // front of the list, and B behind it. Place 1 hands over to the
        // other value; place 0 keeps the value.
        let packed = record(&[(b'A', 3), (b'B', 2)], &[0, 0, 1, 1, 1]);
        assert_eq!(unpack(&packed).unwrap(), b"AABAB");
    }

    #[test]
    fn a_record_that_is_cut_short_gives_nothing() {
        assert!(unpack(&[]).is_none());
        assert!(unpack(&[0; HEADER_BYTES + 10]).is_none());
        let mut packed = record(&[(b'A', 3)], &[0, 0, 0]);
        packed.truncate(packed.len() - 3);
        assert!(unpack(&packed).is_none());
    }
}
