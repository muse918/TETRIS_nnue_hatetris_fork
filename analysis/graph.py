import matplotlib.pyplot as plt


def replay_name(gen, idx):
    return f"replay/replay_w10_{gen}_{idx}.txt"


generations = list(range(20))
nums = 5

xs = []
ys = []
avgx = []
avgy = []

for i in generations:
    score_sum = 0
    for j in range(nums):
        with open(replay_name(i, j), "r") as f:
            score = int(f.readline().rstrip())
            score_sum += score
            xs.append(i)
            ys.append(score)
    avgx.append(i)
    avgy.append(score_sum / nums)


fig, ax = plt.subplots()
ax.plot(xs, ys, "+")
ax.plot(avgx, avgy)
ax.set_xlabel("Generations", fontsize=20)
ax.set_ylabel("Average score", fontsize=20)
ax.set_title("Average score for generations (width 10)", fontsize=20)
ax.set_xticks(list(range(0, 21, 5)))
plt.show()
