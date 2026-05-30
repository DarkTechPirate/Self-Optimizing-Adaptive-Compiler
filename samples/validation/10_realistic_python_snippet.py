def realistic_backend_logic():
    total = 0
    retries = 0

    for i in range(0, 60):
        total = total + i

        if total > 500:
            retries = retries + 1
        else:
            retries = retries + 0

    return total + retries
