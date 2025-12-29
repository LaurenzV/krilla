# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

import os
import sys

# Point to Python source for pure Python modules
sys.path.insert(0, os.path.abspath('../../python'))

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'krilla'
copyright = '2025, krilla contributors'
author = 'krilla contributors'
release = '0.1.0'

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = [
    'autoapi.extension',            # Auto-generate API docs from source/stubs
    'sphinx.ext.napoleon',          # Google/NumPy style docstrings
    'sphinx.ext.intersphinx',       # Link to other docs
    'sphinx.ext.viewcode',          # Add source code links
    'myst_parser',                  # Markdown support
]

# AutoAPI settings
autoapi_dirs = ['../../python/krilla']
autoapi_type = 'python'
autoapi_options = [
    'members',
    'undoc-members',
    'show-inheritance',
    'show-module-summary',
    'imported-members',
]
autoapi_ignore = ['*/__pycache__/*']
autoapi_keep_files = False
autoapi_add_toctree_entry = False  # We'll manage the TOC manually

templates_path = ['_templates']
exclude_patterns = []

language = 'en'

# MyST parser settings
myst_enable_extensions = [
    'colon_fence',      # ::: fences for directives
    'deflist',          # Definition lists
    'attrs_inline',     # Inline attributes
]
source_suffix = {
    '.rst': 'restructuredtext',
    '.md': 'markdown',
}

# Napoleon settings for Google-style docstrings
napoleon_google_docstring = True
napoleon_numpy_docstring = False
napoleon_include_init_with_doc = True
napoleon_use_param = True
napoleon_use_rtype = True

# AutoAPI template directory (optional customization)
autoapi_template_dir = '_autoapi_templates'

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = 'sphinx_rtd_theme'
html_theme_options = {
    'navigation_depth': 4,
    'collapse_navigation': False,
}
html_static_path = ['_static']

# Intersphinx - link to other docs
intersphinx_mapping = {
    'python': ('https://docs.python.org/3', None),
}
