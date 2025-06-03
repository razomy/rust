fn main() {
    // 1. Define the dimensions of our 3D world
    let world_depth = 10; // Size along Z-axis
    let world_height = 10; // Size along Y-axis
    let world_width = 10;  // Size along X-axis

    // 2. Create the 3D array, initialized to `false` (empty)
    // The structure is world[z][y][x]
    let mut world: Vec<Vec<Vec<bool>>> =
        vec![vec![vec![false; world_width]; world_height]; world_depth];

    // 3. Define the cube's parameters
    let cube_start_x = 2;
    let cube_start_y = 2;
    let cube_start_z = 2;
    let cube_side_length = 3; // This will make a 3x3x3 cube

    println!(
        "Defining a cube starting at ({}, {}, {}) with side length {}",
        cube_start_x, cube_start_y, cube_start_z, cube_side_length
    );

    // 4. Write the cube into the 3D array
    // Iterate through the conceptual volume of the cube
    for z_offset in 0..cube_side_length {
        for y_offset in 0..cube_side_length {
            for x_offset in 0..cube_side_length {
                // Calculate the actual coordinates in the world
                let x = cube_start_x + x_offset;
                let y = cube_start_y + y_offset;
                let z = cube_start_z + z_offset;

                // Boundary check: ensure the cube's coordinates are within the world
                if x < world_width && y < world_height && z < world_depth {
                    world[z][y][x] = true; // Mark this cell as part of the cube
                } else {
                    // Optionally, handle cases where the cube goes out of bounds
                    // For this example, we'll just skip parts outside the world.
                    // println!(
                    //     "Warning: Cube coordinate ({}, {}, {}) is outside world bounds!",
                    //     x, y, z
                    // );
                }
            }
        }
    }

    // 5. Print the coordinates of the cells that are part of the cube
    println!("\nCoordinates of the cells forming the cube:");
    for z in 0..world_depth {
        for y in 0..world_height {
            for x in 0..world_width {
                if world[z][y][x] { // Check if this cell is part of the cube
                    println!("({}, {}, {})", x, y, z);
                }
            }
        }
    }

    // Optional: A simple visual representation (for small worlds)
    if world_width <= 20 && world_height <= 20 && world_depth <= 20 {
        println!("\nSimple 2D slice view (Z-slices):");
        for z in 0..world_depth {
            println!("--- Z = {} ---", z);
            for y in 0..world_height {
                for x in 0..world_width {
                    if world[z][y][x] {
                        print!(" #"); // Cube cell
                    } else {
                        print!(" ."); // Empty cell
                    }
                }
                println!(); // Newline for next row in Y
            }
            println!(); // Extra newline between Z-slices
        }
    }
}