def redundant_hotpath():
    total = 0
    bonus = 0

    for i in range(0, 100):
        calc_a = i * 2
        calc_b = i * 2
        total = total + calc_a

        if total > 250:
            bonus = bonus + calc_b

    return total + bonus
