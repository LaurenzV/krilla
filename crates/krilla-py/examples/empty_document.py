"""This example contains the most basic document you can create: A document
with a single empty page.
"""

from pathlib import Path

from krilla import Document, PageSettings


def main():
    # First, we create a new document. This represents a single PDF document.
    document = Document()
    # We can now successively add new pages by calling `start_page`, or
    # `start_page_with` if we want to pass custom page settings.
    with document.start_page_with(PageSettings.from_wh(300.0, 600.0)):
        pass

    # Create the PDF
    pdf = document.finish()

    path = Path("empty_document.pdf").absolute()
    print(f"Saved PDF to '{path}'")

    # Write the PDF to a file.
    with open(path, "wb") as f:
        f.write(pdf)


if __name__ == "__main__":
    main()
