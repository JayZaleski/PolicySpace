"""Build the manual: manual.ipynb -> manual.html and manual.pdf.

Run from the project root:

    .venv/Scripts/python tools/build_manual.py

The notebook is executed once, with its code cells hidden in both outputs.
The HTML is nbconvert's usual export, styled by manual.css, which the
notebook's first cell embeds. The PDF is nbconvert's LaTeX export, typeset by
XeLaTeX with the template in tools/manual_pdf/, which matches manual.css. The
Markdown cells go through pandoc (from the pypandoc_binary package), and the
SVG figures become PDF through tools/manual_pdf/svg2pdf.py. XeLaTeX must be on
the PATH (TinyTeX or any TeX Live); everything else comes from requirements.txt.
"""

import os
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NOTEBOOK = ROOT / "manual.ipynb"
HTML = ROOT / "manual.html"
PDF = ROOT / "manual.pdf"
TEMPLATES = ROOT / "tools"
SVG2PDF = ROOT / "tools" / "manual_pdf" / "svg2pdf.py"


def prepare_path():
    """Put pandoc on the PATH and check for xelatex."""
    import pypandoc

    pandoc_dir = str(Path(pypandoc.get_pandoc_path()).parent)
    os.environ["PATH"] = pandoc_dir + os.pathsep + os.environ.get("PATH", "")
    if not shutil.which("xelatex"):
        sys.exit("xelatex is not on the PATH. Install TinyTeX (or TeX Live) to build the PDF.")


def execute(nb):
    from nbconvert.preprocessors import ExecutePreprocessor

    print("> executing manual.ipynb", flush=True)
    ExecutePreprocessor(timeout=600).preprocess(nb, {"metadata": {"path": str(ROOT)}})


def export(nb):
    from nbconvert import HTMLExporter, PDFExporter
    from traitlets.config import Config

    config = Config()
    # The same as nbconvert's --no-input.
    config.TemplateExporter.exclude_input = True
    config.TemplateExporter.exclude_input_prompt = True
    config.TemplateExporter.exclude_output_prompt = True
    config.PDFExporter.template_name = "manual_pdf"
    config.PDFExporter.extra_template_basedirs = [str(TEMPLATES)]
    config.SVG2PDFPreprocessor.command = [
        sys.executable, str(SVG2PDF), "{from_filename}", "{to_filename}",
    ]
    resources = {"metadata": {"name": NOTEBOOK.stem}}

    print("> writing manual.html", flush=True)
    body, _ = HTMLExporter(config=config).from_notebook_node(nb, resources=dict(resources))
    HTML.write_text(body, encoding="utf-8")

    print("> writing manual.pdf (xelatex)", flush=True)
    data, _ = PDFExporter(config=config).from_notebook_node(nb, resources=dict(resources))
    PDF.write_bytes(data)
    print(f"wrote {HTML.name} ({HTML.stat().st_size:,} bytes) and {PDF.name} ({PDF.stat().st_size:,} bytes)")


def main(argv):
    if argv:
        sys.exit(__doc__)
    import nbformat

    os.chdir(ROOT)
    prepare_path()
    nb = nbformat.read(NOTEBOOK, as_version=4)
    execute(nb)
    export(nb)


if __name__ == "__main__":
    main(sys.argv[1:])
