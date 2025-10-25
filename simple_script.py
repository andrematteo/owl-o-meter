import time
import sys

print("... Started... ")
time.sleep(2)

result = sum(range(1000000))
print(f"Result..: {result}")

print("End.. !")
sys.exit(0)