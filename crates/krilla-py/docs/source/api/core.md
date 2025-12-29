# Core Types

The core types provide the main API for creating PDF documents.

## Document

```{eval-rst}
.. autoclass:: krilla.Document
   :members:
   :undoc-members:
   :show-inheritance:
   :special-members: __init__, __enter__, __exit__
```

The Document class is the main entry point for PDF creation. It manages the overall document structure and serialization context.

## Page

```{eval-rst}
.. autoclass:: krilla.Page
   :members:
   :undoc-members:
   :show-inheritance:
   :special-members: __enter__, __exit__
```

Represents a single page in the PDF document. Pages are created through `Document.start_page()` and automatically finished when exiting the context manager.

## Surface

```{eval-rst}
.. autoclass:: krilla.Surface
   :members:
   :undoc-members:
   :show-inheritance:
   :special-members: __enter__, __exit__
```

The drawing surface provides methods for rendering paths, text, and images. Surfaces are created through `Page.surface()` and support context managers for graphics state operations like transforms, blend modes, and clipping.

## PageSettings

```{eval-rst}
.. autoclass:: krilla.PageSettings
   :members:
   :undoc-members:
   :show-inheritance:
```

Configuration for page dimensions and properties.
