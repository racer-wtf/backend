-- Add down migration script here
drop table if exists block_heights;

drop table if exists votes;

drop table if exists cycles;

drop domain if exists address;

drop domain if exists bytes4;

drop domain if exists uint256;

drop domain if exists uint128;

drop domain if exists uint64;

drop domain if exists uint56;
