-- Add up migration script here
create domain address as char(42)
check (VALUE ~ '^0x[a-f0-9]{40}$');

create domain bytes4 as bytea check (length(value) <= 4);

create domain uint256 as numeric(78, 0);

create domain uint128 as numeric(39, 0);

create domain uint64 as numeric(20, 0);

create domain uint56 as numeric(17, 0);

create table if not exists cycles (
  id uint256 primary key,
  chain_id uint256 not null,
  block_number uint64 not null,
  creator address not null,
  starting_block uint64 not null,
  block_length uint64 not null,
  vote_price uint256 not null,
  balance uint128 not null,
  "current" boolean not null default false
);

create table if not exists votes (
  id uint256 primary key,
  chain_id uint256 not null,
  block_number uint64 not null,
  cycle_id uint256 not null references cycles (id),
  symbol bytes4 not null,
  claimed boolean not null default false,
  amount uint56 not null,
  placer address not null
);

create table if not exists block_heights (
  chain_id uint256 primary key,
  height uint256 not null
);
