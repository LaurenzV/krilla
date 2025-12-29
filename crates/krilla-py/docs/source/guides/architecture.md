# Architecture & Design

Understanding the krilla Python bindings architecture and design decisions.

## Overview

The krilla Python bindings provide a safe, Pythonic interface to the Rust krilla PDF library while maintaining the strict ownership and lifetime guarantees of the underlying Rust API.

## Ownership Chain

The core krilla Rust API has a strict ownership chain:

```text
Document → Page<'doc> → Surface<'page>
```

Where:
- `Page` mutably borrows the serialization context from `Document`
- `Surface` mutably borrows the serialization context from `Page`
- Only one Page can exist at a time (enforced at compile time in Rust)
- Only one Surface can exist at a time (enforced at compile time in Rust)

This ensures single-threaded exclusive access to the serialization context and enforces proper nesting of page/surface operations.

## Python Bindings Architecture

The Python bindings replicate Rust's compile-time guarantees using runtime checks:

- **Lightweight Python wrappers** (`Document`, `Page`, `Surface`) that don't directly own Rust objects
- **Centralized storage** that holds raw pointers to active Rust objects
- **Runtime checks** that replicate the borrow checker's safety guarantees

## Why Runtime Checks?

Python has no compile-time lifetimes. The Rust API returns:
- `Page<'doc>` where `'doc` is the lifetime of the Document borrow
- `Surface<'page>` where `'page` is the lifetime of the Page borrow

To bridge this gap, the Python bindings must extend lifetimes to `'static` internally while enforcing the constraints through runtime checks instead.

## Safety Mechanisms

Safety is maintained through several mechanisms:

### 1. State Flags

Internal state flags track active objects:
- `has_active_page` - Whether a page is currently active
- `has_active_surface` - Whether a surface is currently active

### 2. Method Guards

Operations check these flags before proceeding:
- `start_page()` fails if a page is already active
- `finish()` fails if child objects are still active
- `surface()` fails if the page is finished

### 3. RAII via Context Managers

Python's `with` statement ensures proper cleanup:

```python
with doc.start_page(PageSettings.from_wh(200, 200)) as page:
    with page.surface() as surface:
        # Drawing operations
    # Surface automatically finished
# Page automatically finished
```

### 4. Drop Handlers

Drop handlers clean up state even if `finish()` isn't explicitly called, preventing leaked resources.

## Safety Invariants

The implementation maintains these invariants:

1. **Non-null pointers** - Internal pointers are never null when objects are active
2. **Correct cleanup order** - Objects are always cleaned up in the correct order: Surface → Page → Document
3. **No overlapping access** - No overlapping mutable access to the serialization context (enforced by state flags)
4. **Lifetime enforcement** - Lifetimes are extended internally but actual validity is enforced by runtime checks
5. **Cleanup guarantee** - Drop handlers ensure cleanup even on exception or early return

## Design Rationale

### Why Not Use Reference Counting?

A more idiomatic PyO3 approach would be to:
- Store Rust objects directly in their Python wrappers via `RefCell<Option<T>>`
- Use `Py<T>` for cross-references (e.g., `Page` holds reference to `Document`)
- Eliminate raw pointers in favor of PyO3's reference counting

**Why this approach was not used:**
- Still requires extending lifetimes (same fundamental issue)
- Still requires the same runtime checks for safety (no improvement)
- Creates circular references (`Page` → `Document` → `Py<Page>`)
- Adds more indirection and complexity
- Current approach is simpler with equivalent safety guarantees

The raw pointer approach, while less idiomatic PyO3, is more direct and avoids the complexity of circular references while still maintaining safety through runtime checks.

## Practical Implications

For users of the library, this architecture means:

1. **Always use context managers** - They ensure proper cleanup and prevent errors
2. **One page/surface at a time** - You can't have multiple pages or surfaces active simultaneously
3. **Proper nesting required** - Surfaces must be finished before pages, pages before the document
4. **Automatic cleanup** - Context managers handle cleanup automatically, but you can call `finish()` explicitly if needed

## Example: Correct Usage

```python
from krilla import Document, PageSettings

doc = Document()

# Correct: Proper nesting with context managers
with doc.start_page(PageSettings.from_wh(200, 200)) as page:
    with page.surface() as surface:
        surface.draw_path(path)
    # Surface automatically finished
# Page automatically finished

# Correct: Multiple pages sequentially
with doc.start_page(PageSettings.from_wh(200, 200)) as page1:
    with page1.surface() as surface:
        surface.draw_path(path)

with doc.start_page(PageSettings.from_wh(200, 200)) as page2:
    with page2.surface() as surface:
        surface.draw_path(other_path)

pdf = doc.finish()
```

## Example: Incorrect Usage

```python
# INCORRECT: Starting a page while another is active
page1 = doc.start_page(PageSettings.from_wh(200, 200))
page2 = doc.start_page(PageSettings.from_wh(200, 200))  # Error!

# INCORRECT: Not using context manager (requires manual finish)
page = doc.start_page(PageSettings.from_wh(200, 200))
surface = page.surface()
surface.draw_path(path)
# Forgot to call surface.finish() and page.finish()
# Will error on doc.finish()

# INCORRECT: Trying to use surface after page is finished
with doc.start_page(PageSettings.from_wh(200, 200)) as page:
    surface = page.surface()
    surface.draw_path(path)
    surface.finish()
# Page is finished here
# surface.draw_path(other_path)  # Would error if attempted
```

## Conclusion

The krilla Python bindings successfully bridge the gap between Rust's compile-time lifetime guarantees and Python's dynamic runtime model. By using runtime checks and context managers, the bindings provide a safe, ergonomic API while maintaining the strong guarantees of the underlying Rust library.
