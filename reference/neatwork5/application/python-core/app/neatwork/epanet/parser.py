from __future__ import annotations

import re
from app.neatwork.epanet.models import (EpanetDataLine, EpanetInpDocument, EpanetSection)


SECTION_PATTERN = re.compile(r"^\[([^\]]+)\]$")

def parse_epanet_inp(content: str) -> EpanetInpDocument:
    """
    Parse EPANET INP syntax without making hydraulic assumptions.
    Unknown sections are retained exactly so future adapters can inspect them.
    """

    document = EpanetInpDocument()
    current_section: EpanetSection | None = None

    for (line_number,original_line) in enumerate(content.splitlines(), start=1):
        raw = original_line.lstrip("\ufeff")
        data_part, separator, comment_part = raw.partition(";")
        data = data_part.strip()
        comment = (comment_part.strip() if separator else None)


        # Section headers are parsed after removing inline comments.
        section_match = SECTION_PATTERN.match(data)

        if section_match:
            section_name = section_match.group(1).strip().upper()
            if section_name == "END":
                document.end_seen = True
                break
            if section_name not in document.sections:
                document.sections[section_name] = EpanetSection(name=section_name)
                document.section_order.append(section_name)

            current_section = document.sections[section_name]

            continue

        if current_section is None:
            document.preamble.append(raw)
            continue

        current_section.raw_lines.append(raw)

        # Empty and comment-only lines remain in raw_lines but do not create data records.
        if not data:
            continue
        current_section.lines.append(
            EpanetDataLine(
                line_number=line_number,
                raw=raw,
                data=data,
                tokens=data.split(),
                comment=comment,
            )
        )
    return document
