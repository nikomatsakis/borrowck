A playground for the proposed non-lexical-lifetime algorithm.

The test directory contains a number of scenarios. Each file includes
comments explaining the scenario. They are written in a very
simplified "NLL" notation, which specifies the control-flow graph, the
local variables, and the relationships between the various regions
involved.

To try it out for yourself:

```
> cd nll
> cargo run ../test/*nll
```

This will run the code against all the test files and verify the
embedded assertions within. You should expect to see all OK results.
