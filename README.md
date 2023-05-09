# Perfect collisions

Project focusing on finding the optimal method for detecting & handling collisions in space.

Cur approach: max 200 polygons of 3-8 sides before frame drop.

Optimization candidate 1: 
- compare only those whose trajectory-bounding boxes intersect on the x-coordinate (>2000 objects)