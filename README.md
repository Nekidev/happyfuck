# Happyfuck

Happyfuck is an esoteric programming language based on Brainfuck, designed to be more user-friendly
and easier to use.

Happyfuck is a superset of brainfuck. All brainfuck code is valid happyfuck code, but not all
happyfuck code is valid brainfuck code. Happyfuck adds new commands and features to make
programming easier and more enjoyable.

This repository contains two interpreters, one for brainfuck and one for happyfuck

## Introduction to Brainfuck Concepts

Brainfuck is a minimalist programming language that operates on an array of memory cells, each
initially set to zero. The language uses a simple set of eight commands to manipulate the memory
and control the flow of the program. The commands are as follows:

| Command | Description                                                                                     |
| ------- | ----------------------------------------------------------------------------------------------- |
| `+`     | Increments the value of the current cell by 1.                                                  |
| `-`     | Decrements the value of the current cell by 1.                                                  |
| `>`     | Moves the memory pointer to the right.                                                          |
| `<`     | Moves the memory pointer to the left.                                                           |
| `.`     | Outputs the ASCII character corresponding to the value of the current cell.                     |
| `,`     | Accepts one byte of input and stores it in the current cell.                                    |
| `[`     | If the value of the current cell is zero, jumps forward to the command after the matching `]`.  |
| `]`     | If the value of the current cell is non-zero, jumps back to the command after the matching `[`. |

In the happyfuck interpreter, the memory is unbounded to the right (up to 2^32 in 32-bit machines
and 2^64 in 64-bit machines) and bounded to the left (the pointer cannot move left of the starting
position). The memory cells are bytes, meaning they can hold values from 0 to 255. When a cell's
value exceeds 255, it wraps around to 0, and when it goes below 0, it wraps around to 255. Going
left at the leftmost cell does not change the pointer position.

For example:

```
# Outputs SEAL in loop

>+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++  # S in ASCII
>+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++                # E in ASCII
>+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++                    # A in ASCII
>++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++         # L in ASCII
<<<<  # Go to the first position

+[            # Output value in loop
    >.>.>.>.  # Output all characters
    <<<<      # Go back to the first position
]
```

> _Note: `#` are not valid brainfuck comments._

In the example above, we have a starting memory where every byte is initialized to zero. We then
skip the first cell and increment the next four cells to the ASCI values of 'S', 'E', 'A', and 'L'.
We then enter a loop that outputs the characters in sequence and returns to the first cell. Since
we never change the first cell's value, the loop repeats forever.

### Brainfuck by Example

Happyfuck builds upon the foundation of brainfuck by introducing new commands and features to make
programming easier and more enjoyable.

To begin with, let's explain with examples how brainfuck works. Memory is an array of bytes, all
initially set to zero. We also have a memory pointer that starts at the first cell.

```
0 0 0 0 0 0 0 0 0 0 0...
^
```

The first command is `+`, which increments the value of the current cell by 1. For example, if we
run `+++++`, the memory will end up looking like this:

```
5 0 0 0 0 0 0 0 0 0 0...
^
```

We can also subtract from the current cell using `-`. If we run `+++++---`, the memory will look
like this:

```
2 0 0 0 0 0 0 0 0 0 0...
^
```

To move the pointer left and right, we use `>` and `<`. For example, if we run `+>++`, the memory
will look like this:

```
1 2 0 0 0 0 0 0 0 0 0...
  ^
```

Great! Now we can write values to memory and move around. To output the value of the current cell
(usually to the console), we use `.`. For example, if we run `+` 65 times followed by `.`, we will
output the character 'A', since 'A' has an ASCII value of 65.

```
+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

65 0 0 0 0 0 0 0 0 0 0...
^^
```

To read input, we use `,`. This command will read a single byte of input and store it in the
current cell. For example, if we run `,>,<.>.`, we'll read 2 bytes of input and output them.

```
# Read 5 bytes of input
,>,>,>,>,

# Input is "Hello"
72 101 108 108 111

# Output all 5 letters
<<<<
.>.>.>.>.
```

Finally, we have loops. Loops are defined using `[` and `]`. A loop will continue to execute as
long as the value of the current cell at the beginning of the iteration is non-zero. For example,
if we run `+++++[>+++++<-]`, the memory will look like this:

```
0 25 0 0 0 0 0 0 0 0 0...
^
```

Iteration by iteration, the loop does the following:

```
+++++
05 00 00 00 00 00 00 00 00 00 00...
^^

[>+++++<-]
04 05 00 00 00 00 00 00 00 00 00...
^^

[>+++++<-]
03 10 00 00 00 00 00 00 00 00 00...
^^

[>+++++<-]
02 15 00 00 00 00 00 00 00 00 00...
^^

[>+++++<-]
01 20 00 00 00 00 00 00 00 00 00...
^^

[>+++++<-]
00 25 00 00 00 00 00 00 00 00 00...
^^
```

## Happyfuck

Happyfuck extends brainfuck by adding new commands and features to make programming easier and more
enjoyable. The new commands are as follows:

| Command | Description                                                                                      |
| ------- | ------------------------------------------------------------------------------------------------ |
| `=`     | Set the current cell to a specific value.                                                        |
| `#`     | Comment. Anything after `#` on the same line is ignored.                                         |
| `~`     | Go to cell at an index.                                                                          |
| `@`     | Target the next write command at another cell.                                                   |
| `$`     | Write the current memory pointer in the current cell.                                            |
| `()`    | Repeat the code inside the parentheses a fixed amount of times.                                  |
| `{}`    | A code block used as an expression. It resolves to the value of the cell the code block ends on. |

Happyfuck brings strings, targeted writing, expressions, simplifications, and more to brainfuck.

The first thing happyfuck does is simplifying common brainfuck patterns.

### Repetition

The simplest example is `+++++`. When you want to write a letter, say the S, you have to repeat the
`+` command 83 times. With happyfuck, you can specify a number after the `+` command to indicate
how many times you want to repeat it.

```
+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
# Becomes
+83
```

The same applies to the `-`, `<`, and `>` commands. Remember all brainfuck code is valid happyfuck
code, so you can still use the old way if you want to.

### Comments

In brainfuck, there is no standard way to write comments. You can use any character that is not a
command, but it's not ideal because 1) it's not standarized, and 2) you cannot use punctuation used
by brainfuck.

In happyfuck, comments are denoted by the `#` symbol. Anything after `#` on the same line is
ignored by the interpreter. This allows you to write comments without worrying about interfering
with your code.

```
# Hey there! This is a comment.
+++
# ---
+++
```

The result of the code above is that the second and fourth lines increment the current cell by 3,
while the third and first lines get ignored.

### Setting Cell Values

Instead of keeping track of how many times you incremented or decremented a cell, you can directly
set the value of a cell using the `=` command. For example, if you want to set the current cell to
83, you can simply write `=83`.

```
=83
```

This command sets the current cell to 83, regardless of its previous value. The same way, you can
reset a cell to zero by writing `=0`.

### Going to an Index

The `~` command allows you to move the memory pointer to a specific index. For example, if you want
to move the pointer to the 10th cell, you can write `~10`.

```
~10
```

### Repeat Loops

You already have loops in brainfuck using `[]`. To loop a fixed amount of times, you can use the
`()` command. The amount of times to repeat the code inside the parentheses is specified after the
`)`.

```
# This code
(>.)5
# Is equivalent to
>.>.>.>.>.
```

# TODO