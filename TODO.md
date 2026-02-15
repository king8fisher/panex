# TODO

## Tunnel Demo
Recursive panex tunnel - spawn nested panex instances to create a visual tunnel effect. Good dogfooding opportunity.

- [x] Foundation implemented: alternate screen clearing, area bleed-through fixes, and mouse forwarding in focus mode.

## End2End Testing
Golden file testing: render terminal buffer to string, compare against snapshots. Use `insta` crate for snapshot management. Test buffer/state layer; skip full visual TUI testing.

## Resource Prioritization
Distribute resources evenly across running processes.

## Mouse Selection
Support text selection with mouse in output panel.

## Nested Panex: Accept Shift-Tab
Forward Shift-Tab from nested panex to parent so it can exit focus mode.

## Adaptive Edge-Scroll Speed
During drag-to-select, measure how fast the cursor moved from one line to the edge before hitting the first/last line. Use that velocity to scale edge-scroll speed instead of a fixed interval.

## Nested Selection Constraint
Add a way to constrain selection to a grandchild app that runs from a child panex.

