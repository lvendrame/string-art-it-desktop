# 07 — Pin Geometry Engine

## Purpose

Define the mathematically precise algorithms that turn a geometric path into a set of pins: open-path distribution, closed-path uniform distribution, continuous spacing through corners, and the precision/performance requirements on the underlying Geometry Engine (see [01-architecture.md](./01-architecture.md) for layer boundaries).

## Open-Path Pin Distribution

Applies to **Arc** and **Freehand** only. Line is vertex-anchored instead (see [Vertex-Anchored Pin Distribution](#vertex-anchored-pin-distribution)) — its two endpoints are always pinned, unlike the floor-division rule below.

For open paths (Arc, Freehand, other non-closed compound paths):

```text
L = path length
D = requested pin spacing
```

Pins occur at:

```text
0
D
2D
3D
...
floor(L / D) × D
```

Pin count:

```text
floor(L / D) + 1
```

The endpoint is **not** artificially forced to contain a pin — any residual distance less than `D` after the last pin remains empty.

### Examples

```text
Length = 8 cm, Spacing = 1 cm
Pins at: 0 1 2 3 4 5 6 7 8
Result: 9 pins
```

```text
Length = 7.5 cm, Spacing = 1 cm
Pins at: 0 1 2 3 4 5 6 7
Result: 8 pins (last 0.5 cm remains empty)
```

## Closed-Path Pin Distribution

Applies to **Circle** and **Ellipse** only. Rectangle, Square, and the whole regular-polygon/Star/Polygram family are vertex-anchored instead (see [Vertex-Anchored Pin Distribution](#vertex-anchored-pin-distribution)).

Closed paths (Circle, Ellipse):

Closed shapes require **uniform pin spacing around the complete perimeter** — there must never be a small residual closing gap.

### Closed-Path Pin Count Algorithm

> Choose the integer pin count that produces an actual uniform spacing closest to the requested spacing.

```text
P = total perimeter
D = requested spacing

N ≈ P / D                    // approximate interval count

Evaluate nearby integer candidates for N.
actualSpacing(N) = P / N

Choose N that minimizes:
  | actualSpacing(N) - D |
```

The number of physical pins for a closed path equals the number of intervals `N` — the start/end coordinate is a single shared pin, so there is no duplicate.

### Example

```text
Perimeter = 31 cm
Requested spacing = 2 cm

Approximate interval count: 31 / 2 = 15.5

15 intervals → 31 / 15 = 2.0667 cm   (diff from 2 cm: 0.0667 cm)
16 intervals → 31 / 16 = 1.9375 cm   (diff from 2 cm: 0.0625 cm)

Chosen: 16 intervals → 16 pins, actual spacing 1.9375 cm
```

The application must expose this result to the user (see live preview below and [05-canvas-and-viewport.md](./05-canvas-and-viewport.md) status bar).

## Continuous Pin Spacing Through Corners

Applies to **Circle** and **Ellipse** only (compound curved paths). These are treated as **one continuous geometric path** — spacing does not restart on each arc segment.

```text
Edge 1
   ↓
─────────┐
         │
         │ Edge 2
         │
```

If the next pin falls 2 mm after a corner, it is positioned 2 mm into the next segment. Distance accumulation continues through every corner without resetting.

Rectangle, Square, and the regular-polygon/Star/Polygram family do **not** use continuous accumulation — every vertex is a hard reset point that always receives a pin (see [Vertex-Anchored Pin Distribution](#vertex-anchored-pin-distribution)).

## Vertex-Anchored Pin Distribution

Applies to **Line, Rectangle, Square, regular polygons (Pentagon/Hexagon/Octagon/...), Stars, Polygrams, and the Path tool's free-form Polygon** ([33-pin-path-tool.md](./33-pin-path-tool.md)) — every shape whose guide path *is* its vertices, where each segment of the path is a real edge between two real corners.

Every vertex always receives a pin. The pins **between** two consecutive vertices are distributed independently along that one edge, using the same closest-integer-interval-count rule as [Closed-Path Pin Distribution](#closed-path-pin-distribution) (`N` chosen so `edgeLength / N` is closest to the requested spacing), scoped to that single edge instead of the whole path. Spacing does **not** carry over across a corner — each edge starts its own closest-N approximation fresh.

For a closed shape (Rectangle, Square, regular polygon, Star, Polygram, Path-tool Polygon), the last edge's end vertex is the first edge's start vertex, so it is counted once — no duplicate seam pin, same guarantee as the closed-path algorithm.

For the one open case (Line, a single edge with two vertices), **both** endpoints are always pinned — this differs from [Open-Path Pin Distribution](#open-path-pin-distribution)'s floor-division rule, which never forces the end pin.

### Example (octagon, vertex-anchored)

```text
Regular octagon, 8 equal edges, requested spacing 2 cm, each edge length 3.5 cm

Each edge: closestIntervalCount(3.5, 2) → N = 2 → actual spacing 1.75 cm
Per edge: 1 vertex pin + 1 interior pin = 2 pins contributed
Total: 8 vertices + 8 interior pins = 16 pins, actual spacing 1.75 cm
```

### Example (Line, vertex-anchored)

```text
Length = 7.5 cm, Requested spacing = 1 cm

closestIntervalCount(7.5, 1) → N = 8 candidates: 7 (spacing 1.071) vs 8 (spacing 0.9375)
  |1.071 - 1| = 0.071, |0.9375 - 1| = 0.0625 → N = 8 chosen
Pins at 0, 0.9375, 1.875, ... , 7.5 (both endpoints included)
Result: 9 pins, actual spacing 0.9375 cm

(Contrast with the old floor-division rule: 8 pins at 0..7, 0.5 cm left empty, no pin at 7.5)
```

## Multi-Contour Closed-Path Distribution (Text)

Applies to **Text** only ([29-text-pin-path.md](./29-text-pin-path.md)) — the one shape whose guide geometry is more than one disconnected closed loop (a letter's ring plus its hole, or a separate dot).

A Text Pin Path's geometry holds a list of contours instead of one continuous path. Each contour is a compound curved path (a font glyph outline, flattened from Bezier curves into a polyline) — not a shape defined by real straight-edge vertices — so it uses the **same rule as Circle/Ellipse**: [Closed-Path Pin Distribution](#closed-path-pin-distribution), continuous accumulation around that one contour's perimeter, closest-integer-interval-count spacing. The rule runs **independently per contour** — spacing never carries over from one contour to the next, since two contours of one letter (e.g. "o"'s outer ring and inner hole) are unrelated perimeters, each uniformly spaced on its own.

The Pin Path's `pins[]` is the flat concatenation of every contour's pins, in contour order. `actualSpacing` reports the first contour's actual spacing as a representative value — the same "one number even though it varies per piece" precedent already established by [Vertex-Anchored Pin Distribution](#vertex-anchored-pin-distribution)'s per-edge shapes above.

### Example

```text
Letter "o", flattened to two contours:
  Outer ring, perimeter 40 cm
  Inner hole, perimeter 16 cm
Requested spacing: 2 cm

Outer ring: closestIntervalCount(40, 2) -> 20 intervals, actual spacing 2.0 cm exactly
Inner hole: closestIntervalCount(16, 2) -> 8 intervals, actual spacing 2.0 cm exactly

Total pins: 20 + 8 = 28
Reported actualSpacing: 2.0 cm (the outer ring's value)
```

## Live Preview Values

While creating or resizing a Pin Path, the editor displays live values that update whenever shape dimensions, rotation, curvature, or requested spacing change.

**Open path:**
```text
Length:          37.4 cm
Requested gap:    1.0 cm
Pins:            38
```

**Closed path:**
```text
Perimeter:        37.4 cm
Requested gap:     1.0 cm
Actual gap:        1.01 cm
Pins:             37
```

The closed-shape preview is always geometrically closed (no visual gap), even while the shape is still being dragged/resized.

## Geometry Precision

Geometry must use mathematical paths rather than screen-pixel approximation:

- Circle → true circumference
- Arc → true arc length
- Ellipse → accurate numerical path length (no closed-form solution — approximate numerically, e.g. via elliptic integral approximation or adaptive numerical integration, to a defined error tolerance)
- Rectangle → exact per-edge length (vertex-anchored, see [Vertex-Anchored Pin Distribution](#vertex-anchored-pin-distribution))
- Polygon → exact per-edge length (vertex-anchored)
- Stars → exact per-edge length (vertex-anchored)
- Polygrams → correct self-intersecting per-edge geometric traversal (vertex-anchored; not the convex hull)

## Geometry Engine Responsibilities (Pin-Generation Scope)

- Path length computation for every supported shape type
- Point-at-distance (given a path and a distance along it, return the point — used by both distribution algorithms)
- Open pin distribution
- Closed pin distribution / spacing optimization
- Continuous accumulation across corners
- Bounding boxes, rotation, transformation (needed to recompute geometry on edit — see [09-selection-and-editing.md](./09-selection-and-editing.md))

## Performance

Nearest-pin detection (used heavily by Thread mode, [12-thread-editor.md](./12-thread-editor.md)) should eventually use a spatial index instead of checking every pin on every pointer-move event. Acceptable implementations: spatial hash, quadtree, R-tree, k-d tree. Exact implementation is not prescribed — this is a non-functional requirement to design the pin-storage/query interface so a spatial index can be introduced later without changing calling code.

## Test Cases

```gherkin
Feature: Open-path pin distribution (Arc, Freehand only — Line is vertex-anchored)

  Scenario: Exact division produces pins including both endpoints
    Given an open Arc path of length 8 cm
    And requested spacing 1 cm
    When pins are distributed
    Then pins are placed at 0,1,2,3,4,5,6,7,8 cm
    And the pin count is 9

  Scenario: Non-exact division leaves a residual gap and does not force an endpoint pin
    Given an open Arc path of length 7.5 cm
    And requested spacing 1 cm
    When pins are distributed
    Then pins are placed at 0,1,2,3,4,5,6,7 cm
    And the pin count is 8
    And no pin is placed at 7.5 cm
    (A Line of the same length and spacing behaves differently — see Vertex-Anchored Pin Distribution below)

  Scenario: Spacing larger than path length still places the start pin
    Given an open Arc path of length 0.5 cm
    And requested spacing 5 cm
    When pins are distributed
    Then exactly 1 pin is placed, at position 0

  Scenario: Very small path with very small spacing
    Given an open Arc path of length 0.01 cm
    And requested spacing 0.001 cm
    When pins are distributed
    Then the pin count equals floor(0.01/0.001) + 1 = 11
    And floating-point rounding does not produce 10 or 12 pins

Feature: Vertex-anchored pin distribution (Line, Rectangle, Square, regular polygons, Stars, Polygrams)

  Scenario: Line forces both endpoints even on non-exact division
    Given a Line of length 7.5 cm
    And requested spacing 1 cm
    When pins are distributed
    Then interval count 8 is chosen for the single edge (closest to spacing 1 cm)
    And pins are placed at 0, 0.9375, 1.875, ..., 7.5 cm
    And the pin count is 9
    And both endpoints (0 cm and 7.5 cm) are pinned

  Scenario: Every vertex of a closed straight-edged shape gets a pin
    Given a regular octagon Pin Path with 8 equal edges of length 3.5 cm
    And requested spacing 2 cm
    When pins are distributed
    Then each edge independently chooses interval count 2 (actual spacing 1.75 cm)
    And all 8 vertices are pinned
    And each edge contributes exactly 1 interior pin
    And the pin count is 16
    And the seam vertex (shared by the last and first edge) is counted once, not twice

  Scenario: An edge shorter than the requested spacing still gets both its vertex pins
    Given a closed straight-edged shape with one edge of length 1 cm
    And requested spacing 5 cm
    When pins are distributed
    Then that edge contributes its 2 vertex pins and 0 interior pins

Feature: Closed-path uniform pin distribution (Circle, Ellipse only)

  Scenario: Perimeter divides the requested spacing to the closer candidate (31/2 example)
    Given a circle with perimeter 31 cm
    And requested spacing 2 cm
    When pins are distributed
    Then the interval count chosen is 16
    And the actual spacing is 1.9375 cm
    And the pin count is 16

  Scenario: Exact closed division produces no rounding artifact
    Given a circle with perimeter 30 cm
    And requested spacing 3 cm
    When pins are distributed
    Then the interval count chosen is 10
    And the actual spacing is exactly 3 cm
    And the pin count is 10

  Scenario: No duplicate pin at the start/end seam
    Given any Circle or Ellipse path with pins distributed
    Then the pin at cumulative distance 0 and the pin at cumulative distance P are the same single pin, not two coincident pins

  Scenario: Closed path chooses the closer of two candidate spacings deterministically
    Given a circle with perimeter 100 cm
    And requested spacing 7 cm
    When comparing interval counts 14 (spacing 7.143 cm) and 15 (spacing 6.667 cm)
    Then interval count 14 is chosen because |7.143-7| < |6.667-7|

  Scenario: Spacing request larger than perimeter still yields a minimum valid closed shape
    Given a circle with perimeter 5 cm
    And requested spacing 20 cm
    When pins are distributed
    Then the algorithm chooses interval count 1
    And the pin count is 1 (a single pin placed at the path start)

Feature: Continuous spacing through corners (Circle, Ellipse only)

  Scenario: Spacing carries over across an ellipse's internal arc-segment boundary
    Given an ellipse approximated as adjacent arc segments, segment 1 length 10 cm and segment 2 length 10 cm (sharing a boundary)
    And requested spacing 3 cm
    When the last pin on segment 1 falls 1 cm before the boundary
    Then the next pin is placed 2 cm into segment 2 (not reset to 0 cm from the boundary)

  Scenario: Rectangle and Octagon do NOT use continuous accumulation
    Given a rectangle or a regular octagon Pin Path
    When pins are distributed with a requested spacing
    Then each edge is treated as an independent segment (see Vertex-Anchored Pin Distribution)
    And every vertex receives a pin regardless of where continuous accumulation would have landed

Feature: Geometry precision

  Scenario: Circle uses true circumference
    Given a circle of diameter 10 cm
    Then its computed perimeter equals π × 10 cm (within floating-point tolerance), not a polygon approximation

  Scenario: Ellipse length is numerically accurate
    Given an ellipse with width 10 cm and height 6 cm
    Then its computed perimeter matches a numerical-integration reference value within a defined tolerance (e.g. 0.1%)

  Scenario: Arc length reflects true curvature
    Given an arc defined by start, end, and curvature producing a specific radius and included angle
    Then its computed length equals radius × includedAngle(radians), not the straight-line chord distance

  Scenario: Polygram perimeter follows self-intersecting traversal
    Given a pentagram (5-pointed star polygon) Pin Path
    Then its computed perimeter follows the star's actual point-to-point traversal path
    And is not equal to the perimeter of its convex hull (the outer pentagon)

Feature: Multi-contour closed-path distribution (Text only)

  Scenario: Each contour is distributed independently
    Given a Text Pin Path with an outer contour of perimeter 40 cm and an inner contour of perimeter 16 cm
    And requested spacing 2 cm
    When pins are distributed
    Then the outer contour receives 20 pins and the inner contour receives 8 pins
    And the total pin count is 28
    And no pin is duplicated at either contour's start/end seam

  Scenario: Spacing does not carry over between contours
    Given a Text Pin Path with two contours
    When the last pin on the first contour falls short of that contour's own perimeter
    Then the second contour's distribution starts fresh at its own distance 0
    And does not continue accumulating distance from the first contour

Feature: Live preview updates

  Scenario: Open-path preview updates as spacing changes
    Given an Arc or Freehand path of length 12.7 cm being drawn
    When the requested gap is set to 1 cm
    Then the preview shows "Length: 12.7 cm | Gap: 1 cm | Pins: 13"
    When the requested gap is changed to 2 cm
    Then the preview updates to show 7 pins

  Scenario: Closed-path preview shows requested vs actual gap live while resizing
    Given a circle Pin Path being resized with requested gap 1 cm
    When the diameter is adjusted such that perimeter becomes 37.4 cm
    Then the preview shows "Perimeter: 37.4 cm | Requested gap: 1.0 cm | Actual gap: 1.01 cm | Pins: 37"
```
