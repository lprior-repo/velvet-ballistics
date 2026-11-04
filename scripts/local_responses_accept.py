#!/usr/bin/env python3
"""Acceptance gate for local Responses API output (vb-spbjo).

The bridge must run this against every raw native response BEFORE
accepting model output as a completion. Native ``status="completed"``
is not sufficient: a token-limited, reasoning-only transcript with an
empty final text is output-budget exhaustion and must propagate as
``incomplete``, never ``completed``.

Usage:
    python3 scripts/local_responses_accept.py RESPONSE.json \
        --max-output-tokens 8192
    python3 scripts/local_responses_accept.py --self-test

Exit codes: 0 accepted, 2 incomplete, 3 rejected, 1 usage error.
Stdout always carries a machine-readable verdict JSON that preserves
reasoning presence, reasoning tokens, and actual usage. This tool is
read-only: it never modifies or deletes the raw request/response files.
"""

import argparse
import json
import sys

EXIT_ACCEPTED = 0
EXIT_INCOMPLETE = 2
EXIT_REJECTED = 3
EXIT_USAGE_ERROR = 1


def _collect_output_text(output):
    parts = []
    if not isinstance(output, list):
        return "", False
    reasoning_present = False
    for item in output:
        if not isinstance(item, dict):
            continue
        if item.get("type") == "reasoning":
            reasoning_present = True
            continue
        if item.get("type") != "message":
            continue
        content = item.get("content")
        if not isinstance(content, list):
            continue
        for block in content:
            if isinstance(block, dict) and block.get("type") == "output_text":
                text = block.get("text")
                if isinstance(text, str):
                    parts.append(text)
    return "".join(parts), reasoning_present


def classify(response, max_output_tokens=None):
    """Classify a decoded native Responses JSON object.

    Returns (verdict, reason, detail dict). Pure function over the
    decoded JSON value plus the requested output budget.
    """
    if not isinstance(response, dict):
        return ("rejected", "unparseable-response", {})
    status = response.get("status")
    usage = response.get("usage")
    if not isinstance(usage, dict):
        usage = {}
    output_tokens = usage.get("output_tokens")
    details = usage.get("output_tokens_details")
    reasoning_tokens = None
    if isinstance(details, dict):
        reasoning_tokens = details.get("reasoning_tokens")
    final_text, reasoning_present = _collect_output_text(response.get("output"))
    detail = {
        "status": status,
        "output_tokens": output_tokens,
        "max_output_tokens": max_output_tokens,
        "reasoning_present": reasoning_present,
        "reasoning_tokens": reasoning_tokens,
        "has_final_text": bool(final_text.strip()),
    }
    if status != "completed":
        if status == "incomplete":
            return ("incomplete", "native-incomplete", detail)
        return ("rejected", "native-not-completed", detail)
    if (
        max_output_tokens is not None
        and isinstance(output_tokens, int)
        and output_tokens >= max_output_tokens
    ):
        return ("incomplete", "output-budget-exhausted", detail)
    if not final_text.strip():
        return ("rejected", "empty-final-output", detail)
    return ("accepted", "completed-with-final-output", detail)


def _verdict_doc(verdict, reason, detail):
    return {
        "verdict": verdict,
        "reason": reason,
        "status": detail.get("status"),
        "usage": {
            "output_tokens": detail.get("output_tokens"),
            "max_output_tokens": detail.get("max_output_tokens"),
            "reasoning_tokens": detail.get("reasoning_tokens"),
        },
        "reasoning_present": detail.get("reasoning_present"),
        "has_final_text": detail.get("has_final_text"),
    }


def _self_test():
    cases = [
        # (name, response, budget, expected_verdict, expected_reason)
        (
            "completed-valid",
            {
                "status": "completed",
                "usage": {"output_tokens": 743},
                "output": [
                    {
                        "type": "message",
                        "role": "assistant",
                        "content": [
                            {"type": "output_text", "text": "Approved."}
                        ],
                    }
                ],
            },
            8192,
            "accepted",
            "completed-with-final-output",
        ),
        (
            "vb-parity-code-shard-a-reasoning-only-budget-hit",
            {
                "status": "completed",
                "usage": {
                    "output_tokens": 8192,
                    "output_tokens_details": {"reasoning_tokens": 8192},
                },
                "output": [{"type": "reasoning", "id": "rs_1"}],
            },
            8192,
            "incomplete",
            "output-budget-exhausted",
        ),
        (
            "completed-empty-text-under-budget",
            {
                "status": "completed",
                "usage": {"output_tokens": 100},
                "output": [],
            },
            8192,
            "rejected",
            "empty-final-output",
        ),
        (
            "native-incomplete",
            {
                "status": "incomplete",
                "usage": {"output_tokens": 8192},
                "output": [],
            },
            8192,
            "incomplete",
            "native-incomplete",
        ),
        (
            "native-failed",
            {"status": "failed", "usage": {}, "output": []},
            8192,
            "rejected",
            "native-not-completed",
        ),
        (
            "unparseable-response",
            ["not", "a", "dict"],
            8192,
            "rejected",
            "unparseable-response",
        ),
    ]
    failures = 0
    for name, response, budget, want_verdict, want_reason in cases:
        verdict, reason, detail = classify(response, budget)
        doc = _verdict_doc(verdict, reason, detail)
        ok = verdict == want_verdict and reason == want_reason
        # Budget-hit case must preserve reasoning evidence and usage.
        if name == "vb-parity-code-shard-a-reasoning-only-budget-hit":
            ok = ok and doc["reasoning_present"] is True
            ok = ok and doc["usage"]["output_tokens"] == 8192
            ok = ok and doc["usage"]["reasoning_tokens"] == 8192
        print(("PASS" if ok else "FAIL") + " " + name + " -> " + verdict + "/" + reason)
        if not ok:
            failures += 1
    print(str(len(cases) - failures) + "/" + str(len(cases)) + " passed")
    return EXIT_USAGE_ERROR if failures else EXIT_ACCEPTED


def main(argv):
    parser = argparse.ArgumentParser(
        description="Acceptance gate for local Responses API output."
    )
    parser.add_argument("response", nargs="?", help="Raw native response JSON file.")
    parser.add_argument(
        "--max-output-tokens",
        type=int,
        default=None,
        help="Requested output budget for exhaustion check.",
    )
    parser.add_argument(
        "--self-test", action="store_true", help="Run fixture matrix and exit."
    )
    args = parser.parse_args(argv)
    if args.self_test:
        return _self_test()
    if not args.response:
        parser.print_usage(sys.stderr)
        return EXIT_USAGE_ERROR
    try:
        with open(args.response, encoding="utf-8") as handle:
            response = json.load(handle)
    except (OSError, ValueError):
        doc = _verdict_doc("rejected", "unparseable-response", {})
        print(json.dumps(doc, sort_keys=True))
        return EXIT_REJECTED
    verdict, reason, detail = classify(response, args.max_output_tokens)
    print(json.dumps(_verdict_doc(verdict, reason, detail), sort_keys=True))
    if verdict == "accepted":
        return EXIT_ACCEPTED
    if verdict == "incomplete":
        return EXIT_INCOMPLETE
    return EXIT_REJECTED


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
