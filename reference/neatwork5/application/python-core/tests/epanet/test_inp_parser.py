from pathlib import Path

from app.neatwork.epanet import (
    parse_epanet_inp,
)


FIXTURE_ROOT = (
    Path(__file__)
    .resolve()
    .parents[1]
    /
    "fixtures"
    /
    "epanet"
)


def test_parser_reads_sections_and_comments():

    content = (
        FIXTURE_ROOT
        /
        "simple_supported.inp"
    ).read_text(
        encoding="utf-8"
    )


    document = (
        parse_epanet_inp(
            content
        )
    )


    assert (
        document.section(
            "JUNCTIONS"
        )
        is not None
    )


    assert (
        document.section(
            "PIPES"
        )
        is not None
    )


    assert (
        document.end_seen
        is True
    )

def test_unknown_sections_are_preserved():

    document = (
        parse_epanet_inp(
            """
[JUNCTIONS]
J1 10 0

[CUSTOM_SECTION]
ABC 123

[END]
"""
        )
    )


    custom = (
        document.section(
            "CUSTOM_SECTION"
        )
    )


    assert custom is not None

    assert (
        custom.lines[0]
        .tokens
        ==
        [
            "ABC",
            "123",
        ]
    )

