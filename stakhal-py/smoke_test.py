import os
import sys
import stakhal_py

fixture_dir = os.path.abspath(
    os.path.join(
        os.path.dirname(__file__),
        "../stakhal-core/tests/fixtures/stm32_03_timers"
    )
)

print(f"Running stakhal-py smoke test on fixture dir: {fixture_dir}")

project = stakhal_py.load_project_from_dir(fixture_dir)

print("\n--- Project Meta ---")
print(f"Name: {project.meta.name}")
print(f"MCU Family: {project.meta.mcu_family}")
print(f"MCU Name: {project.meta.mcu_name}")

print("\n--- PV Declarations ---")
pv_list = project.pv_declarations
print(f"Found {len(pv_list)} PV declarations:")
for decl in pv_list:
    print(f"  - {decl.name}: {decl.type_str} (line {decl.line})")

print("\n--- Call Graph ---")
edges = project.call_graph_edges
layout = stakhal_py.compute_graph_layout(edges, [])
print(f"Call Graph Edge Count: {len(edges)}")
print(f"Call Graph Node Count (in layout): {len(layout.positions)}")
print(f"Graph Bounds: {layout.bounds}")

print("\nSmoke test PASSED!")
