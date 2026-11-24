//! Runtime support for array operations using ArrayHeader API
//!
//! ArrayHeader layout (32 bytes total):
//! - magic: u64 (8 bytes) - corruption detection
//! - capacity: usize (8 bytes)
//! - len: usize (8 bytes)
//! - canary: u64 (8 bytes) - overflow detection
//!
//! Data follows immediately after the header.
//! array_new returns pointer to data (after header), not to header.
#![allow(unsafe_code)]

use std::alloc::{Layout, alloc, dealloc, realloc};
use std::ptr;

/// ArrayHeader structure for bulletproof memory system
#[repr(C)]
struct ArrayHeader {
    magic: u64,      // Corruption detection (0xARRAYMAGIC)
    capacity: usize, // Maximum number of elements
    len: usize,      // Current number of elements
    canary: u64,     // Overflow detection (0xDEADBEEF)
}

const ARRAY_HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();
const MAGIC_VALUE: u64 = 0x41525241; // "ARRA" in hex for corruption detection
const CANARY_VALUE: u64 = 0xDEADBEEFCAFEBABE; // 64-bit canary for overflow detection

// Memory sanitization patterns
const UNINITIALIZED_PATTERN: u8 = 0xCD; // Pattern for uninitialized memory
const FREED_PATTERN: u8 = 0xFD; // Pattern for freed memory

/// Get header from data pointer
unsafe fn get_header(data_ptr: *mut i64) -> *mut ArrayHeader {
    unsafe { (data_ptr as *mut u8).sub(ARRAY_HEADER_SIZE) as *mut ArrayHeader }
}

/// Get data pointer from header
unsafe fn get_data(header: *mut ArrayHeader) -> *mut i64 {
    unsafe { (header as *mut u8).add(ARRAY_HEADER_SIZE) as *mut i64 }
}

/// Byte layout of an array block holding `capacity` elements.
fn block_layout(capacity: usize) -> Option<Layout> {
    let data_size = capacity.checked_mul(std::mem::size_of::<i64>())?;
    let total_size = ARRAY_HEADER_SIZE.checked_add(data_size)?;
    let align = std::cmp::max(
        std::mem::align_of::<ArrayHeader>(),
        std::mem::align_of::<i64>(),
    );
    Layout::from_size_align(total_size, align).ok()
}

/// Grow an array so it can hold `new_capacity` elements.
///
/// Header and data share one block, and `array_new` hands the caller a pointer
/// *into* that block (the data pointer). `realloc` may move the block, so this
/// returns the new data pointer; `array_push` passes it back to the language-side
/// variable, keeping exactly one authoritative pointer.
///
/// Returns `None` when the allocation cannot be satisfied, leaving the array
/// untouched.
unsafe fn grow_array(header: *mut ArrayHeader, new_capacity: usize) -> Option<*mut i64> {
    if header.is_null() {
        return None;
    }

    let old_capacity = unsafe { (*header).capacity };
    if new_capacity <= old_capacity {
        return Some(unsafe { get_data(header) });
    }

    let old_layout = block_layout(old_capacity)?;
    let new_layout = block_layout(new_capacity)?;

    let new_block = unsafe { realloc(header as *mut u8, old_layout, new_layout.size()) };
    if new_block.is_null() {
        return None;
    }

    let new_header = new_block as *mut ArrayHeader;
    unsafe {
        (*new_header).capacity = new_capacity;
        let data_ptr = get_data(new_header);
        // Sanitize the freshly added tail so callers never observe stale bytes.
        let old_end = (data_ptr as *mut u8).add(old_capacity * std::mem::size_of::<i64>());
        let added = (new_capacity - old_capacity) * std::mem::size_of::<i64>();
        ptr::write_bytes(old_end, UNINITIALIZED_PATTERN, added);
        Some(data_ptr)
    }
}

/// Check if header is valid
unsafe fn check_header(header: *const ArrayHeader) -> bool {
    if header.is_null() {
        return false;
    }

    // Check magic value
    unsafe { (*header).magic == MAGIC_VALUE }
}

/// Check if canary is intact
unsafe fn check_canary(header: *const ArrayHeader) -> bool {
    if header.is_null() {
        return false;
    }

    unsafe { (*header).canary == CANARY_VALUE }
}

/// Create a new array with given capacity
///
/// # Safety
/// Returns a pointer to data (after ArrayHeader), not to header.
/// Returns 0 when the allocation cannot be satisfied.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn array_new(capacity: usize) -> i64 {
    // Calculate total size: header + data
    let elem_size = std::mem::size_of::<i64>();
    let data_size = match capacity.checked_mul(elem_size) {
        Some(size) => size,
        None => return 0,
    };
    let total_size = match ARRAY_HEADER_SIZE.checked_add(data_size) {
        Some(size) => size,
        None => return 0,
    };

    // Allocate single contiguous block for header + data
    let align = std::cmp::max(
        std::mem::align_of::<ArrayHeader>(),
        std::mem::align_of::<i64>(),
    );
    let layout = match Layout::from_size_align(total_size, align) {
        Ok(layout) => layout,
        Err(_) => {
            return 0;
        }
    };

    let block_ptr = unsafe { alloc(layout) } as *mut u8;
    if block_ptr.is_null() {
        return 0;
    }

    // Initialize header
    let header_ptr = block_ptr as *mut ArrayHeader;
    unsafe {
        (*header_ptr).magic = MAGIC_VALUE;
        (*header_ptr).capacity = capacity;
        (*header_ptr).len = 0;
        (*header_ptr).canary = CANARY_VALUE;
    }

    // Get data pointer (after header)
    let data_ptr = unsafe { get_data(header_ptr) };

    // Initialize data with sanitization pattern (0xCD for uninitialized)
    unsafe {
        ptr::write_bytes(data_ptr as *mut u8, UNINITIALIZED_PATTERN, data_size);
    }

    // Return data pointer (not header pointer)
    data_ptr as i64
}

/// Get array length
///
/// # Safety
/// ptr must be a valid data pointer returned by array_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn array_len(ptr: i64) -> i64 {
    if ptr == 0 {
        return 0;
    }

    let data_ptr = ptr as *mut i64;
    let header = unsafe { get_header(data_ptr) };

    // Check header integrity
    if !unsafe { check_header(header) } {
        // Magic corrupted - memory corruption detected
        println!(
            "[ARRAY_LEN] Expected magic: 0x{:X}, Found: 0x{:X}",
            MAGIC_VALUE,
            unsafe { (*header).magic }
        );
        return -1; // Error code for memory corruption
    }

    // Check canary for overflow detection
    if !unsafe { check_canary(header) } {
        // Canary corrupted - buffer overflow detected
        println!(
            "[ARRAY_LEN] Expected canary: 0x{:X}, Found: 0x{:X}",
            CANARY_VALUE,
            unsafe { (*header).canary }
        );
        return -2; // Error code for buffer overflow
    }

    unsafe { (*header).len as i64 }
}

/// Get element at index
///
/// # Safety
/// ptr must be a valid data pointer returned by array_new, index must be < length
#[unsafe(no_mangle)]
pub unsafe extern "C" fn array_get(ptr: i64, index: i64) -> i64 {
    if ptr == 0 {
        return 0;
    }

    let data_ptr = ptr as *mut i64;

    // Check if this is a stack array (no header)
    // For stack arrays, the pointer points directly to data
    // We can't reliably detect stack arrays, so we'll try to read the header
    // If the magic is wrong, we assume it's a stack array
    let header = unsafe { get_header(data_ptr) };

    // Check header integrity
    if unsafe { check_header(header) } {
        // This is a heap array with a valid header

        // Check canary for overflow detection
        if !unsafe { check_canary(header) } {
            // Canary corrupted - buffer overflow detected
            println!(
                "[ARRAY_GET] Expected canary: 0x{:X}, Found: 0x{:X}",
                CANARY_VALUE,
                unsafe { (*header).canary }
            );
            return -2; // Error code for buffer overflow
        }

        // Bounds checking
        let len = unsafe { (*header).len };
        let capacity = unsafe { (*header).capacity };
        println!(
            "[ARRAY_GET] Array length = {}, capacity = {}",
            len, capacity
        );
        if index < 0 || index as usize >= len {
            // Index out of bounds
            println!(
                "[ARRAY_GET] ERROR: Index out of bounds! Index={}, Length={}",
                index, len
            );
            return -3; // Error code for index out of bounds
        }
        if index as usize >= capacity {
            // Index exceeds capacity (should never happen if length <= capacity)
            println!(
                "[ARRAY_GET] ERROR: Index exceeds capacity! Index={}, Capacity={}",
                index, capacity
            );
            return -4; // Error code for capacity exceeded
        }

        unsafe { *data_ptr.offset(index as isize) }
    } else {
        // This is a stack array (no header)
        // Just read the value directly
        // WARNING: No bounds checking for stack arrays!
        // The compiler should generate bounds checking for stack arrays
        let value = unsafe { *data_ptr.offset(index as isize) };
        println!(
            "[ARRAY_GET] Stack array value at index {}: {}",
            index, value
        );
        value
    }
}

/// Set element at index
///
/// # Safety
/// ptr must be a valid data pointer returned by array_new, index must be < length
#[unsafe(no_mangle)]
pub unsafe extern "C" fn array_set(ptr: i64, index: i64, value: i64) {
    if ptr == 0 {
        return;
    }

    let data_ptr = ptr as *mut i64;

    // Check if this is a stack array (no header)
    let header = unsafe { get_header(data_ptr) };

    // Check header integrity
    if unsafe { check_header(header) } {
        // This is a heap array with a valid header

        // Check canary for overflow detection
        if !unsafe { check_canary(header) } {
            // Canary corrupted - buffer overflow detected
            println!(
                "[ARRAY_SET] Expected canary: 0x{:X}, Found: 0x{:X}",
                CANARY_VALUE,
                unsafe { (*header).canary }
            );
            return;
        }

        // Bounds checking
        let len = unsafe { (*header).len };
        let capacity = unsafe { (*header).capacity };
        println!(
            "[ARRAY_SET] Array length = {}, capacity = {}",
            len, capacity
        );
        if index < 0 || index as usize >= len {
            // Index out of bounds
            println!(
                "[ARRAY_SET] ERROR: Index out of bounds! Index={}, Length={}",
                index, len
            );
            return;
        }
        if index as usize >= capacity {
            // Index exceeds capacity (should never happen if length <= capacity)
            println!(
                "[ARRAY_SET] ERROR: Index exceeds capacity! Index={}, Capacity={}",
                index, capacity
            );
            return;
        }

        unsafe {
            *data_ptr.offset(index as isize) = value;
        }
    } else {
        // Stack array (no header): the caller supplies the element address
        // directly. A negative index would address memory before the array, so
        // refuse it instead of writing outside the buffer.
        if index < 0 {
            println!(
                "[ARRAY_SET] ERROR: negative index {} for stack array",
                index
            );
            return;
        }
        unsafe {
            *data_ptr.offset(index as isize) = value;
        }
    }
}

/// Push element to array
///
/// # Safety
/// ptr must be a valid data pointer returned by array_new (or by this function).
///
/// Returns the data pointer to keep using. The array grows geometrically and
/// growth may relocate the block, so the caller must store the return value back
/// into the variable it took the pointer from. Returns the incoming pointer
/// unchanged when the push was refused (null array, corrupted header, stack
/// array, or allocation failure).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn array_push(ptr: i64, value: i64) -> i64 {
    if ptr == 0 {
        return ptr;
    }

    let mut data_ptr = ptr as *mut i64;
    let header = unsafe { get_header(data_ptr) };

    if !unsafe { check_header(header) } {
        // A stack array carries no length/capacity metadata, so `array_push`
        // cannot know where the next free slot is. Guessing (previously: always
        // writing to index 0) silently corrupted data, so refuse the operation
        // and report it instead of writing to the wrong slot.
        //
        // Stack arrays must be initialized with `stack_array_set(ptr, index,
        // value)`, which does take an explicit index.
        println!(
            "[ARRAY_PUSH] ERROR: cannot push to a stack array (no length metadata); use stack_array_set with an explicit index"
        );
        return ptr;
    }

    if !unsafe { check_canary(header) } {
        // Canary corrupted - buffer overflow detected
        println!(
            "[ARRAY_PUSH] Expected canary: 0x{:X}, Found: 0x{:X}",
            CANARY_VALUE,
            unsafe { (*header).canary }
        );
        return ptr;
    }

    let len = unsafe { (*header).len };
    let capacity = unsafe { (*header).capacity };

    if len >= capacity {
        // Grow geometrically: new = old * 1.5 + 1 (see
        // docs/04_ADVANCED_FEATURES_TUTORIAL.md). Growth may move the block.
        let new_capacity = capacity + capacity / 2 + 1;
        match unsafe { grow_array(header, new_capacity) } {
            Some(new_data) => data_ptr = new_data,
            None => {
                println!(
                    "[ARRAY_PUSH] ERROR: failed to grow array beyond capacity {}",
                    capacity
                );
                return ptr;
            }
        }
    }

    let header = unsafe { get_header(data_ptr) };
    let len = unsafe { (*header).len };
    unsafe {
        *data_ptr.add(len) = value;
        (*header).len = len + 1;
    }

    data_ptr as i64
}

/// Set array length
///
/// # Safety
/// ptr must be a valid data pointer returned by array_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn array_set_len(ptr: i64, len: i64) {
    if ptr == 0 {
        return;
    }

    let data_ptr = ptr as *mut i64;
    let header = unsafe { get_header(data_ptr) };

    // Check header integrity
    if unsafe { check_header(header) } {
        // This is a heap array with a valid header
        let capacity = unsafe { (*header).capacity };
        if len >= 0 && (len as usize) <= capacity {
            unsafe {
                (*header).len = len as usize;
            }
        } else {
            println!(
                "[ARRAY_SET_LEN] Invalid len {} for capacity {}",
                len, capacity
            );
        }
    }
    // For stack arrays, we don't have a length field
}

/// Free array
///
/// # Safety
/// ptr must be a valid data pointer returned by array_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn array_free(ptr: i64) {
    if ptr == 0 {
        return;
    }

    let data_ptr = ptr as *mut i64;
    let header = unsafe { get_header(data_ptr) };

    // Calculate total size: header + data
    let capacity = unsafe { (*header).capacity };
    let elem_size = std::mem::size_of::<i64>();
    let data_size = capacity * elem_size;
    let total_size = ARRAY_HEADER_SIZE + data_size;

    // Get the original allocation pointer (start of header)
    let block_ptr = header as *mut u8;

    // Fill memory with freed pattern (0xFD) before deallocation
    unsafe {
        ptr::write_bytes(block_ptr, FREED_PATTERN, total_size);
    }

    // Deallocate the entire block
    let align = std::cmp::max(
        std::mem::align_of::<ArrayHeader>(),
        std::mem::align_of::<i64>(),
    );
    if let Ok(layout) = Layout::from_size_align(total_size, align) {
        unsafe { dealloc(block_ptr, layout) };
    }
}

/// Alias for stack_array_get
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stack_array_get(ptr: i64, index: i64) -> i64 {
    unsafe { array_get(ptr, index) }
}

/// Alias for stack_array_set
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stack_array_set(ptr: i64, index: i64, value: i64) {
    unsafe { array_set(ptr, index, value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Full lifecycle: allocate -> push -> get/set -> len -> free.
    #[test]
    fn dynamic_array_lifecycle() {
        unsafe {
            let ptr = array_new(2);
            assert_ne!(ptr, 0, "array_new must return a usable pointer");
            assert_eq!(array_len(ptr), 0, "new array starts empty");

            let ptr = array_push(ptr, 7);
            let ptr = array_push(ptr, 9);
            assert_eq!(array_len(ptr), 2);
            assert_eq!(array_get(ptr, 0), 7);
            assert_eq!(array_get(ptr, 1), 9);

            array_set(ptr, 1, 42);
            assert_eq!(array_get(ptr, 1), 42);
            assert_eq!(array_len(ptr), 2, "array_set must not change len");

            array_free(ptr);
        }
    }

    /// `array_new` never hands back a shared placeholder address.
    #[test]
    fn array_new_returns_distinct_pointers() {
        unsafe {
            let first = array_new(4);
            let second = array_new(4);
            assert_ne!(first, 0x0BADF00D);
            assert_ne!(first, second, "each allocation must be distinct");
            assert_eq!(array_len(first), 0);
            assert_eq!(array_len(second), 0);
            array_free(first);
            array_free(second);
        }
    }

    /// Out-of-bounds and negative indices are rejected instead of written.
    #[test]
    fn array_set_rejects_out_of_bounds_index() {
        unsafe {
            let ptr = array_new(4);
            let ptr = array_push(ptr, 1);

            array_set(ptr, 5, 99);
            array_set(ptr, -1, 99);

            assert_eq!(array_len(ptr), 1);
            assert_eq!(array_get(ptr, 0), 1);
            array_free(ptr);
        }
    }

    /// Pushing past the initial capacity grows the array instead of failing,
    /// and data written before the growth stays readable afterwards.
    #[test]
    fn array_push_grows_geometrically() {
        unsafe {
            let mut ptr = array_new(1);
            let initial_capacity = (*get_header(ptr as *mut i64)).capacity;
            let total = initial_capacity + 20;

            for value in 0..total {
                ptr = array_push(ptr, value as i64);
                assert_ne!(ptr, 0, "a growing push must keep returning a live pointer");
            }

            assert_eq!(array_len(ptr), total as i64);
            for value in 0..total {
                assert_eq!(array_get(ptr, value as i64), value as i64);
            }
            array_free(ptr);
        }
    }

    /// Freeing a null pointer is a no-op rather than an invalid dealloc.
    #[test]
    fn array_free_accepts_null() {
        unsafe { array_free(0) };
    }
}
