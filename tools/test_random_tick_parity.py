#!/usr/bin/env python3
"""Compare vanilla random ticks (and scheduled ticks) with horizon-world, state by state.

Every block state is put at the origin of a random neighbourhood and `randomTick`
(or `tick`) is run with a seeded RNG on both sides; the resulting block changes,
scheduled ticks, side effects and the RNG fingerprint after the call must match.
The `--crop-growth` mode adds focused cases for supported age-stage, tree, vertical and spreading plants.
The Rust probe defaults to the release profile; set `HORIZON_PARITY_PROFILE=fast`
to use the optimized fast profile with shorter link times during local iteration.
"""
import random
try:
    from . import block_update_harness as h
except ImportError:
    import block_update_harness as h


def cases(facts, count, seed, only):
    rng = random.Random(seed)
    pool = h.state_pool(facts, only)
    for n in range(count):
        state = pool[n % len(pool)]
        case = {**h.environment(rng), 'pos': [0, 0, 0], 'cells': facts.cells(rng, state),
                'mode': 'random' if n % 4 else 'tick'}
        yield case


def crop_growth_cases(facts, per_block, seed):
    """Focused random ticks for every block with a crop-grow age stage.

    `per_block` cases are generated for each target, cycling through its block
    states so age/half variants receive comparable oracle coverage. Sapling
    oracle cases use stage zero because advancing a stage-one sapling requires
    dynamic worldgen registries that the map-backed Java oracle does not load;
    live Rust growth tests cover actual tree placement.
    """
    rng = random.Random(seed)
    targets = (
        'wheat', 'carrots', 'potatoes', 'beetroots', 'torchflower_crop',
        'pumpkin_stem', 'melon_stem', 'cocoa', 'nether_wart',
        'sweet_berry_bush', 'pitcher_crop', 'cactus', 'sugar_cane',
        'red_mushroom', 'brown_mushroom',
        'bamboo', 'bamboo_sapling', 'vine', 'chorus_flower', 'kelp',
        'weeping_vines', 'twisting_vines', 'cave_vines',
        'oak_sapling', 'spruce_sapling',
        'birch_sapling', 'jungle_sapling', 'acacia_sapling', 'cherry_sapling',
        'dark_oak_sapling', 'pale_oak_sapling', 'poplar_sapling',
        'mangrove_propagule',
    )
    saplings = {
        'oak_sapling', 'spruce_sapling', 'birch_sapling', 'jungle_sapling',
        'acacia_sapling', 'cherry_sapling', 'dark_oak_sapling',
        'pale_oak_sapling', 'poplar_sapling', 'mangrove_propagule',
    }
    for block in targets:
        pool = [i for i, state in enumerate(facts.states) if state[0] == f'minecraft:{block}']
        if block in saplings:
            pool = [
                i for i in pool
                if facts.states[i][4].get('properties', {}).get('stage') == '0'
            ]
        if not pool:
            raise ValueError(f'no states found for crop-growth target {block}')
        for n in range(per_block):
            state = pool[n % len(pool)]
            state_info = facts.states[state]
            if block == 'mangrove_propagule' and state_info[4].get('properties', {}).get('hanging') == 'true':
                cells = [[0, 0, 0, state]]
            else:
                cells = facts.cells(rng, state)
            yield {
                **h.environment(rng), 'pos': [0, 0, 0],
                'cells': cells, 'mode': 'random',
            }


def main():
    parser = h.argument_parser(__doc__)
    parser.add_argument(
        '--crop-growth', action='store_true',
        help='run `--count` random-tick cases for every crop-grow target block',
    )
    args = parser.parse_args()
    facts = h.load_facts()
    if args.crop_growth:
        selected = crop_growth_cases(facts, args.count, args.seed)
        name = 'crop-growth-random-tick-parity'
    else:
        selected = cases(facts, args.count, args.seed, args.only)
        name = 'random-tick-parity'
    differing = h.execute(name, list(selected), args, facts)
    h.finish(differing)


if __name__ == '__main__':
    main()
