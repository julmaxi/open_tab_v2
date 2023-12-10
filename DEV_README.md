
## Orphaned Ballots

Be careful when disassociating ballots from debates.
The way the schema is set up, it is possible to create a ballot with neither a room,
nor a backup ballot.
This means it is possible to orphan ballots, which not only bloats the DB,
but will also prevent syncs from zero, since these ballots have no associated tournament.

