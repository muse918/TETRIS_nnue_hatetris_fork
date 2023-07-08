import subprocess

# offset = 5 * 13 + 4
offset = 0

data_range = [(i, j) for i in range(0, 20) for j in range(0, 5)]
data_range = data_range[offset:]

subprocess.call(["mkdir", "replay"])

for idx, (i, j) in enumerate(data_range):
    print(f"gen {i} try {j}")
    try:
        subprocess.call(["target/release/hatetris-public", f"{i}", f"w10_{i}_{j}"])
    except KeyboardInterrupt:
        print("To continue: ")
        print(f"offset = {offset + idx}")
        break
