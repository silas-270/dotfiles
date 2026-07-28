import re

with open("src/main.rs", "r") as f:
    content = f.read()

content = content.replace("pool,\n        compositor_state,", "pool,\n        backdrop_pool: smithay_client_toolkit::shm::slot::SlotPool::new(wenv.shm().usize(), &qh).expect(\"shm pool\"),\n        compositor_state,")

with open("src/main.rs", "w") as f:
    f.write(content)

