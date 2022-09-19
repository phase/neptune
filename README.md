# neptune

This tool builds server setups from yaml files. It was
inspired by the systems used at TheArchon and Mineteria,
but instead of loose Python scripts or a giant Kubernetes
nightmare we've got this.

It's a basic Rust application that parses the yaml files
and copies the appropriate plugin and server files into a
folder. The goal is to replace manual jar and config moving
with one config file.

License: MIT

