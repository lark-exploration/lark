The design of the Lark internals is based on [Salsa](https://github.com/salsa-rs/salsa), a framework for doing incremental computation. 

# Salsa

Effectively, Salsa tracks facts as related to each other via indices into sets of tables. Interaction with Salsa happens through sets of user-defined queries that describe both how to do the query as well as what tables will be affected. As queries are run, their answers are cached and made available to successive queries. If an entry in the database has been updated, the cache that contains the old data is invalidated so that it may be recalculated when its needed.

# Entities

An important concept internally is the concept of `entities`. Currently, an ‘entity’ is defined as a source of a definition. They included function declarations, struct declarations, field declarations, and internal symbols and types.

If we query for a definition or the types inside a function, we often use entity-based queries to get this information.

# Tables

The results of queries can also have side tables as part of the result. Let’s take a look at some of these side tables in `FnBody`, the result of the HIR query that gives us the function’s body with symbols resolved to their uses.

```Rust
    pub struct FnBody {
      // ..
      pub tables: FnBodyTables,
    }
```

A look at the `FnBodyTables`:

```Rust
    pub struct FnBodyTables {
      // ..
      pub places: IndexVec<Place, PlaceData>,
    }
```

You’ll see many other tables here, each collecting data about some aspect of the function body (variables, identifiers, expressions, etc).

Each table has a similar shape: a special type that acts as the index (here: `Place`), and another that is the payload (here: `PlaceData`).  The index gives us a lightweight way to refer to the data without copying the payload around.


# DebugWith

Of mention is the internal spacial Debug functionality called DebugWith. DebugWith does a bit more work than the usual Debug functionality in Rust. In addition to giving the contents of the current function, it’s able to traverse the tables to print the rest of the payload needed to display the full item’s contents.

# Compilation

Lark compilation goes through familiar compilation stages, though with the addition of the Salsa system.

First, we open the Lark file inside of the query system. This makes it available to later queries.

Next, we run a query for what we need. For full compilation, this query may query out the generated code. In order to do this, the “code generation” query will itself call queries that call previous stages of queries.  To help clarify, let’s look at an example in pseudo-code:

```
    Start:
      Open file "main.lark"
      
    Code generation:
      For each function:
        Query function body
        Query types of function contents
        Generate equivalent Rust code for typechecked function
      For each struct:
        Query struct contents
        Generate equivalent Rust code for struct
        
    Typecheck function:
      Query function body
      Enumerate through expessions, checking type safety
      
    Function body:
      Lex and light-parse source, looking for matching function
      Parse matching function
      Resolve uses to their definitions
```

With each `query`, we’re likely doing another query (or multiple queries) in the steps to accomplish the task. This allows the whole system to lazily “pull” at the database, reducing unnecessary computation to get the requested answer.

# Parsing

[to be written]

# Type checking and permissions checking

[to be written]

# IDE support

IDE support uses the existing compiler Salsa queries to build up the Language Server operations (or “ls_ops”). These include functionality traverse code looking for references to a given definition, track from a use to its definition, or rename all references — to name a few examples. 

To accomplish this, the Lark system is stood up in “IDE” mode, where it readies the Salsa system and waits for a file to be opened in the IDE. As the IDE opens files, each is sent to the Lark system and then checked for errors. This will give the user the familiar red-squiggle feedback.

When the user types, these edits become updates to the source being stored in the Lark system. Currently, edits will invalidate the whole file contents, though minimizing edit impact is an area of future investigation. Edits will also invalidate any queries currently being run on the previous data, and will attempt to cancel them before the results are sent to the IDE.

IDE requests like find-all-references happen in multiple steps. First, the current source is checked against where the file position where the user is making the request (called the “hover target”). Using the hover target, we can then traverse the functions tables looking for possible references. Rather than crawling the AST, Lark uses many Salsa-based side tables. One such table contains a list of Places (references/uses of an entity or variable).  This linearity helps with cache friendliness and cuts down on traversing less useful nodes.

# Interpreter

The interpreter will query the HIR, and traverse it. As it does so, it creates Values, a enum that contains the possible runtime resulting value. 

Function calls, because they have already been resolved to the function definition in the HIR, can be invoked directly. To do so, the interpreter creates a fresh stack “frame” which is later popped when then function has completed.

Structs are stored as hashmaps, and updated similarly to traditional scripting objects.

# REPL

The REPL works by invoking the HIR translation before invoking the interpreter. By doing so, it can track what HIR node is currently being interpreted. As the user types in a line in the REPL, it is interpreted, and the last interpreted node is stored. When the user types in the next line, the REPL will skip all nodes up to and including the last interpreted node, and then begin interpreting from this node onward.
