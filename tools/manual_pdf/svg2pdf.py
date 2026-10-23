"""Convert one SVG file to PDF, for nbconvert's SVG2PDFPreprocessor.

    python tools/manual_pdf/svg2pdf.py <from.svg> <to.pdf>

The manual's figures are SVGs drawn by the notebook; LaTeX needs them as PDF.
svglib and reportlab do the conversion in pure Python, so no Inkscape is needed.
"""

import sys

from reportlab.graphics import renderPDF
from svglib.svglib import svg2rlg


def main(argv):
    if len(argv) != 2:
        sys.exit(__doc__)
    drawing = svg2rlg(argv[0])
    if drawing is None:
        sys.exit(f"could not read {argv[0]}")
    renderPDF.drawToFile(drawing, argv[1])


if __name__ == "__main__":
    main(sys.argv[1:])
