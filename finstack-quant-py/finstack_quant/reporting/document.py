"""Composition layer: assemble a header, KPI strip, and sections into HTML.

A :class:`TearSheet` renders a scoped fragment for Jupyter (``_repr_html_``) and a
standalone document (``to_html`` / ``save``). Output is fully deterministic: the
CSS scope class is constant and the ``generated`` stamp is caller-injectable.

Examples:
--------
>>> from finstack_quant.reporting.document import TearSheet
>>> from finstack_quant.reporting.theme import INSTITUTIONAL
>>> TearSheet(INSTITUTIONAL, "Demo", []).to_html().startswith("<!DOCTYPE html>")
True
"""

from __future__ import annotations

from dataclasses import dataclass, field
import datetime as dt
import os
from pathlib import Path

from . import format as fmt
from .theme import Theme


def _resolve_sections(
    sections: list[str] | None,
    valid_sections: list[str],
    *,
    valid_label: str = "valid sections",
) -> list[str]:
    wanted = sections if sections is not None else valid_sections
    unknown = set(wanted) - set(valid_sections)
    if unknown:
        raise ValueError(f"unknown section(s): {sorted(unknown)}; {valid_label}: {valid_sections}")
    return wanted


_SCOPE = "fq-ts"

_TOOLTIP_JS = """<script>
(function(){
  function init(){
    document.querySelectorAll('.fq-ts svg .fq-hb').forEach(function(band){
      if(band.__fqWired){return;} band.__fqWired=true;
      var root=band.closest('.fq-ts'); if(!root){return;}
      var tip=root.querySelector('.fq-tip');
      var svg=band.ownerSVGElement;
      var cross=svg&&svg.querySelector('.fq-cross');
      var mk=svg&&svg.querySelector('.fq-mk');
      function show(e){
        if(tip){tip.textContent=band.getAttribute('data-label')+' · '+band.getAttribute('data-val');
          tip.style.left=(e.clientX+12)+'px'; tip.style.top=(e.clientY+12)+'px'; tip.style.opacity='1';}
        var cx=band.getAttribute('data-cx'), cy=band.getAttribute('data-cy');
        if(cross){cross.setAttribute('x1',cx); cross.setAttribute('x2',cx); cross.style.visibility='visible';}
        if(mk){mk.setAttribute('cx',cx); mk.setAttribute('cy',cy); mk.style.visibility='visible';}
      }
      function hide(){ if(tip){tip.style.opacity='0';} if(cross){cross.style.visibility='hidden';} if(mk){mk.style.visibility='hidden';} }
      band.addEventListener('mousemove',show);
      band.addEventListener('mouseenter',show);
      band.addEventListener('mouseleave',hide);
    });
  }
  if(document.readyState==='loading'){document.addEventListener('DOMContentLoaded',init);}else{init();}
})();
</script>"""


@dataclass
class KPI:
    """A single headline statistic in the KPI strip.

    Examples:
    --------
    >>> from finstack_quant.reporting.document import KPI
    >>> KPI("PV", "1.2m USD").label
    'PV'
    """

    label: str
    value: str
    cls: str = ""  # "pos" | "neg" | ""


@dataclass
class Section:
    """A titled block of body HTML, optionally with a subtitle line.

    Examples:
    --------
    >>> from finstack_quant.reporting.document import Section
    >>> Section("Summary", "<p>Ready</p>").title
    'Summary'
    """

    title: str
    body: str
    subtitle: str | None = None


@dataclass
class TearSheet:
    """A composed report. Renders to scoped-fragment or standalone HTML.

    Examples:
    --------
    >>> from finstack_quant.reporting.document import TearSheet
    >>> from finstack_quant.reporting.theme import INSTITUTIONAL
    >>> TearSheet(INSTITUTIONAL, "Demo", []).to_html().startswith("<!DOCTYPE html>")
    True
    """

    theme: Theme
    title: str
    sections: list[Section]
    eyebrow: str = ""
    subtitle: str | None = None
    meta_lines: list[str] = field(default_factory=list)
    kpis: list[KPI] = field(default_factory=list)
    generated: dt.date | None = None
    footer_left: str = ""
    footer_right: str = "finstack-quant"

    def _header_html(self) -> str:
        meta = list(self.meta_lines)
        if self.generated is not None:
            meta = [*meta, f"Generated {fmt.fmt_date(self.generated)}"]
        meta_html = "<br>".join(fmt._escape_html(m) for m in meta)
        sub = f'<div class="subtitle">{fmt._escape_html(self.subtitle)}</div>' if self.subtitle else ""
        return (
            '<div class="head"><div>'
            f'<div class="eyebrow">{fmt._escape_html(self.eyebrow)}</div>'
            f'<div class="title">{fmt._escape_html(self.title)}</div>{sub}</div>'
            f'<div class="meta">{meta_html}</div></div>'
        )

    def _kpis_html(self) -> str:
        if not self.kpis:
            return ""
        cells = "".join(
            f'<div class="kpi"><div class="lbl">{fmt._escape_html(k.label)}</div>'
            f'<div class="val {k.cls}">{fmt._escape_html(k.value)}</div></div>'
            for k in self.kpis
        )
        return f'<div class="kpis">{cells}</div>'

    def _sections_html(self) -> str:
        out = []
        for sec in self.sections:
            out.append(f'<div class="secttl">{fmt._escape_html(sec.title)}</div>')
            if sec.subtitle:
                out.append(f'<p class="sub">{fmt._escape_html(sec.subtitle)}</p>')
            out.append(sec.body)
        return "".join(out)

    def _footer_html(self) -> str:
        return (
            f'<div class="foot"><span>{fmt._escape_html(self.footer_left)}</span>'
            f"<span>{fmt._escape_html(self.footer_right)}</span></div>"
        )

    def _body_fragment(self) -> str:
        return (
            f'<div class="{_SCOPE}">'
            f"{self._header_html()}{self._kpis_html()}{self._sections_html()}{self._footer_html()}"
            '<div class="fq-tip" aria-hidden="true"></div>'
            "</div>"
        )

    def _repr_html_(self) -> str:
        """Scoped fragment for inline Jupyter display."""
        return self.theme.to_css(_SCOPE) + self._body_fragment() + _TOOLTIP_JS

    def to_html(self) -> str:
        """Full standalone HTML document.

        Returns:
        -------
        str
            Complete standalone HTML document containing the configured report.
        """
        return (
            "<!DOCTYPE html>\n"
            '<html lang="en"><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width, initial-scale=1">'
            f"<title>{fmt._escape_html(self.title)}</title>{self.theme.to_css(_SCOPE)}</head>"
            '<body style="margin:0;padding:24px;background:#e8e9ec;">'
            f"{self._body_fragment()}{_TOOLTIP_JS}</body></html>\n"
        )

    def save(self, path: str | os.PathLike[str]) -> None:
        """Write the standalone document to a UTF-8 file.

        Parameters
        ----------
        path : str or os.PathLike[str]
            Destination HTML file path; existing content is replaced.

        Raises:
        ------
        OSError
            If the destination cannot be opened or the HTML cannot be written.

        """
        with Path(path).open("w", encoding="utf-8") as fh:
            fh.write(self.to_html())
