
## Orphaned Ballots

Be careful when disassociating ballots from debates.
The way the schema is set up, it is possible to create a ballot with neither a room,
nor a backup ballot.
This means it is possible to orphan ballots, which not only bloats the DB,
but will also prevent syncs from zero, since these ballots have no associated tournament.


## Authentication

The api server supports two kinds of authentication schemes: User/Password and refresh tokens and access tokens.
User/Password auth can create refresh tokens. Refresh tokens can create access tokens. Access tokens can not create any tokens and expire after ten minutes.
