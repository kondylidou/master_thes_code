# BDD_SAT_Solver

This project was developed in terms of a masterthesis. 

## Project Description

In this project BDD diagrams are used to enhance the performance of a CDCL Solver, Glucose. Glucoses' latest release, namely Glucose 4.1 syrup is used with very few changes. The focus lays on BDD diagrams, which are implemented in Rust and used as a preprocessing technique to make the SAT Solver more efficient and competitive. 

## Language and Communication

As Glucose is implemented in C++ a connection was made and unsafe Rust was used to be able to send the parsed data to the Solver. The BDDs are implemented in Rust and act as a preprocessing(inproccesing?) technique for the Glucose Solver.

## Memory Requirements
## Statistics
## Benchmarks and Testing
