# Perfect collisions

Project focusing on finding the optimal method for detecting & handling collisions in space.

Cur approach: max 2000 polygons of 3-8 sides in near vicinity at 60fps before frame drop, otherwise 15k
The most time when objects are close is taken by detailed collision checking of the candidates, therefore further upper bound lowering could benefit.

Optimization 1: 
- compare only those whose trajectory-bounding boxes intersect on the x-coordinate - when objects close to each other: max. 2000, otherwise the limit is probably like 15k

Optimization 2:
- for those that intersect on x, check whether they also intersect on y
- this should be performed only if objects are too often near each other, otherwise diminishing returns
