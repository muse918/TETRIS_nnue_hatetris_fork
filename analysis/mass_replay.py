import subprocess

offset = 12

data_range = [(i, j) for i in range(0, 19) for j in range(0, 10)]
data_range = data_range[offset:]

subprocess.call(["mkdir", "replay"])

for idx, (i, j) in enumerate(data_range):
    print(f"gen {i} try {j}")
    try:
        subprocess.call(["target/release/hatetris-public", f"{i}", f"{i}_{j}"])
    except KeyboardInterrupt:
        print("To continue: ")
        print(f"offset = {offset + idx}")
        break
