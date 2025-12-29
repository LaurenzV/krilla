# Metadata & Interchange

Document metadata, outline/navigation, and file embedding functionality.

## Document Metadata

```{eval-rst}
.. autoclass:: krilla.Metadata
   :members:
   :undoc-members:
   :show-inheritance:
```

```{eval-rst}
.. autoclass:: krilla.DateTime
   :members:
   :undoc-members:
   :show-inheritance:
```

```{eval-rst}
.. autoclass:: krilla.MetadataTextDirection
   :members:
   :undoc-members:
   :show-inheritance:
```

```{eval-rst}
.. autoclass:: krilla.PageLayout
   :members:
   :undoc-members:
   :show-inheritance:
```

## Document Outline

```{eval-rst}
.. autoclass:: krilla.Outline
   :members:
   :undoc-members:
   :show-inheritance:
```

```{eval-rst}
.. autoclass:: krilla.OutlineNode
   :members:
   :undoc-members:
   :show-inheritance:
```

```{eval-rst}
.. autoclass:: krilla.XyzDestination
   :members:
   :undoc-members:
   :show-inheritance:
```

## File Embedding

```{eval-rst}
.. autoclass:: krilla.EmbeddedFile
   :members:
   :undoc-members:
   :show-inheritance:
```

```{eval-rst}
.. autoclass:: krilla.MimeType
   :members:
   :undoc-members:
   :show-inheritance:
```

```{eval-rst}
.. autoclass:: krilla.AssociationKind
   :members:
   :undoc-members:
   :show-inheritance:
```

## Document Methods

The {py:class}`krilla.Document` class provides these metadata and interchange methods:

- {py:meth}`~krilla.Document.set_metadata` - Set document metadata
- {py:meth}`~krilla.Document.set_outline` - Set navigation outline
- {py:meth}`~krilla.Document.embed_file` - Embed a file in the PDF
- {py:meth}`~krilla.Document.set_location` - Set location for subsequent operations
- {py:meth}`~krilla.Document.reset_location` - Reset the current location

## Example Usage

### Setting Metadata

```python
import krilla

# Create metadata
metadata = (
    krilla.Metadata()
    .title("My Document")
    .authors(["Author Name"])
    .language("en-US")
    .creator("My Application")
)

doc = krilla.Document()
doc.set_metadata(metadata)
```

### Creating an Outline

```python
import krilla

# Create outline with navigation
outline = krilla.Outline()

# Add chapters
chapter1 = krilla.OutlineNode(
    "Chapter 1",
    krilla.XyzDestination(0, krilla.Point.from_xy(100, 700))
)
outline.push_child(chapter1)

doc = krilla.Document()
doc.set_outline(outline)
```

### Embedding Files

```python
import krilla

# Embed a file
mime_type = krilla.MimeType("text/plain")
embedded = krilla.EmbeddedFile(
    "data.txt",
    b"File contents here",
    mime_type=mime_type,
    description="Supporting data",
    association_kind=krilla.AssociationKind.Supplement
)

doc = krilla.Document()
success = doc.embed_file(embedded)  # Returns False if name collision
```
