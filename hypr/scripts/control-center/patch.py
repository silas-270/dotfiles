import re

with open("src/main.rs", "r") as f:
    content = f.read()

# Add backdrop_pool
content = content.replace("pool: SlotPool,", "pool: SlotPool,\n    backdrop_pool: SlotPool,")
content = content.replace("pool: SlotPool::new(wenv.shm().usize(), &qh).expect(\"shm pool\"),", 
                          "pool: SlotPool::new(wenv.shm().usize(), &qh).expect(\"shm pool\"),\n        backdrop_pool: SlotPool::new(wenv.shm().usize(), &qh).expect(\"shm pool\"),")

# Change draw_backdrop to use backdrop_pool
content = content.replace("match self.pool.create_buffer(\n            w as i32,\n            h as i32,\n            stride,\n            wl_shm::Format::Argb8888,\n        )", 
                          "match self.backdrop_pool.create_buffer(\n            w as i32,\n            h as i32,\n            stride,\n            wl_shm::Format::Argb8888,\n        )")

with open("src/main.rs", "w") as f:
    f.write(content)

