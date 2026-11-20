from __future__ import annotations

from app.neatwork.epanet.adapter import (adapt_epanet_to_neatwork)
from app.neatwork.epanet.models import (EpanetAdapterOptions)
from app.neatwork.epanet.parser import (parse_epanet_inp)
from app.neatwork.epanet.semantic import (parse_epanet_semantics)


def parse_and_adapt_epanet(*, name: str, content: str, options: EpanetAdapterOptions | None = None,):
    document = parse_epanet_inp(content)
    semantic, parse_issues = parse_epanet_semantics(document)
    result = adapt_epanet_to_neatwork(semantic, options=options, name=name)
    result.report.issues[0:0] = parse_issues
    result.report.finalize()

    return result