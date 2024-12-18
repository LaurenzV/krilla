A couple of notes for people interested in contributing

# Adding new features
There are many PDF features which have only been introduced at a specific version. In addition to that, some PDF
features are also restricted to specific sub-standards of PDF, like PDF/UA. Because of this, we need to be careful
when adding such new features that they are no in violation of that. In particular, in case a feature is only available
for a specific version, you need to add a check in the code so that it's only used when the current version we are exporting
to supports it.

In `crates/krilla/src/validation`, you can find some documentation on each of the validated export modes, as well as
all the requirements needed to fulfill it, and how Typst fulfills them. So if you don't have access to the original
specification documents, you should check those files to see whether they appear in it, and if so add appropriate checks
to make sure we do not produce documents violating some of the standards.

# Testing
As mentioned in the README, krilla has two kinds of tests: snapshot tests and visual regression tests.
You can create such tests by annotating your tests with the corresponding attributes (see the existing
tests for some examples). The macros are currently somewhat undocumented and not very cleanly implemented, 
but you should be able to find everything you need by looking at examples of existing tests.

By default, if you run `cargo test`, only the snapshot tests will be run, all visual regression tests will
be skipped. This should hopefully be enough for most cases, as running the visual regression tests requires
a very specific setup on your computer. 

If you really do need to run the visual regression tests, you can run
`VISREG="" cargo test`. `krilla` uses my own small library called `sitro` (https://github.com/LaurenzV/sitro) that
I built for this purpose to render PDFs, which basically is an abstraction over different PDF viewers and allows
rendering PDF to bitmap images. However, the prerequisite is that you have the programs installed on your system and
set the corresponding environment variables. You can take a look at the `ci.yml` file to see what the basic necessary
setup is. On MacOS systems, you need to build another additional program that uses Quartz to render PDFs. See the README
of `sitro` for more information on how to do that. As you can imagine, setting this up is quite a pain, but hopefully,
for most cases it should be enough to just run the snapshot tests.

In addition to that, you can prepend `SKIP_SVG` to skip the SVG-related visual regression tests. You can prepent
`REPLACE` to overwrite the currently existing references images/snapshots, and you can use `STORE` to store the
final PDF version of each tests in the `store` directory, which makes it easier to manually inspect them.
