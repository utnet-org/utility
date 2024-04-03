import { useId } from 'react';
import { useQuery } from 'react-query';
import { Tooltip } from 'react-tooltip';
import { ValidatorKickoutReason, fetchEpochInfo } from './api';
import './EpochValidatorsView.scss';

interface ProducedAndExpected {
    produced: number;
    expected: number;
}

type ValidatorRole = 'BlockProducer' | 'ChunkOnlyProducer' | 'None';

interface CurrentValidatorInfo {
    pledge: number;
    shards: number[];
    blocks: ProducedAndExpected;
    chunks: ProducedAndExpected;
}

interface NextValidatorInfo {
    pledge: number;
    shards: number[];
}

interface ValidatorInfo {
    accountId: string;
    current: CurrentValidatorInfo | null;
    next: NextValidatorInfo | null;
    proposalPledge: number | null;
    kickoutReason: ValidatorKickoutReason | null;
    roles: ValidatorRole[];
}

class Validators {
    validators: Map<string, ValidatorInfo> = new Map();

    constructor(private numEpochs: number) {}

    validator(accountId: string): ValidatorInfo {
        if (this.validators.has(accountId)) {
            return this.validators.get(accountId)!;
        }
        const roles = [] as ValidatorRole[];
        for (let i = 0; i < this.numEpochs; i++) {
            roles.push('None');
        }
        this.validators.set(accountId, {
            accountId,
            current: null,
            next: null,
            proposalPledge: null,
            kickoutReason: null,
            roles,
        });
        return this.validators.get(accountId)!;
    }

    setValidatorRole(accountId: string, epochIndex: number, role: ValidatorRole) {
        const validator = this.validator(accountId);
        validator.roles[epochIndex] = role;
    }

    sorted(): ValidatorInfo[] {
        const validators = [...this.validators.values()];
        function sortingKey(info: ValidatorInfo) {
            if (info.current !== null) {
                return [0, -info.current.pledge];
            }
            if (info.next !== null) {
                return [1, -info.next.pledge];
            }
            if (info.proposalPledge !== null) {
                return [2, -info.proposalPledge];
            }
            return [3, 0];
        }
        validators.sort((a, b) => {
            const [ax, ay] = sortingKey(a);
            const [bx, by] = sortingKey(b);
            if (ax == bx) {
                return ay - by;
            }
            return ax - bx;
        });
        return validators;
    }
}

type EpochValidatorViewProps = {
    addr: string;
};

export const EpochValidatorsView = ({ addr }: EpochValidatorViewProps) => {
    const {
        data: epochData,
        error: epochError,
        isLoading: epochIsLoading,
    } = useQuery(['epochInfo', addr], () => fetchEpochInfo(addr));

    if (epochIsLoading) {
        return <div>Loading...</div>;
    }
    if (epochError) {
        return <div className="error">{(epochError as Error).stack}</div>;
    }
    let maxPledge = 0,
        totalPledge = 0,
        maxExpectedBlocks = 0,
        maxExpectedChunks = 0;
    const epochs = epochData!.status_response.EpochInfo;
    const validators = new Validators(epochs.length);
    const currentValidatorInfo = epochData!.status_response.EpochInfo[1].validator_info;
    for (const validatorInfo of currentValidatorInfo.current_validators) {
        const validator = validators.validator(validatorInfo.account_id);
        const pledge = parseFloat(validatorInfo.pledge);
        validator.current = {
            pledge,
            shards: validatorInfo.shards,
            blocks: {
                produced: validatorInfo.num_produced_blocks,
                expected: validatorInfo.num_expected_blocks,
            },
            chunks: {
                produced: validatorInfo.num_produced_chunks,
                expected: validatorInfo.num_expected_chunks,
            },
        };
        maxPledge = Math.max(maxPledge, pledge);
        totalPledge += pledge;
        maxExpectedBlocks = Math.max(maxExpectedBlocks, validatorInfo.num_expected_blocks);
        maxExpectedChunks = Math.max(maxExpectedChunks, validatorInfo.num_expected_chunks);
    }
    for (const validatorInfo of currentValidatorInfo.next_validators) {
        const validator = validators.validator(validatorInfo.account_id);
        validator.next = {
            pledge: parseFloat(validatorInfo.pledge),
            shards: validatorInfo.shards,
        };
    }
    for (const proposal of currentValidatorInfo.current_proposals) {
        const validator = validators.validator(proposal.account_id);
        validator.proposalPledge = parseFloat(proposal.pledge);
    }
    for (const kickout of currentValidatorInfo.prev_epoch_kickout) {
        const validator = validators.validator(kickout.account_id);
        validator.kickoutReason = kickout.reason;
    }
    epochs.forEach((epochInfo, index) => {
        for (const chunkOnlyProducer of epochInfo.chunk_only_producers) {
            validators.setValidatorRole(chunkOnlyProducer, index, 'ChunkOnlyProducer');
        }
        for (const blockProducer of epochInfo.block_producers) {
            validators.setValidatorRole(blockProducer.account_id, index, 'BlockProducer');
        }
    });

    return (
        <table className="epoch-validators-table">
            <thead>
                <tr>
                    <th></th>
                    <th colSpan={4}>Next Epoch</th>
                    <th colSpan={5}>Current Epoch</th>
                    <th colSpan={1 + epochs.length - 2}>Past Epochs</th>
                </tr>
                <tr>
                    <th>Validator</th>

                    <th className="small-text">Role</th>
                    <th className="small-text">Shards</th>
                    <th>Pledge</th>
                    <th>Proposal</th>

                    <th className="small-text">Role</th>
                    <th className="small-text">Shards</th>
                    <th>Pledge</th>
                    <th>Blocks</th>
                    <th>Chunks</th>

                    <th>Kickout</th>
                    {epochs.slice(2).map((epoch) => {
                        return (
                            <th key={epoch.epoch_id} className="small-text">
                                {epoch.epoch_id.substring(0, 4)}...
                            </th>
                        );
                    })}
                </tr>
            </thead>
            <tbody>
                {validators.sorted().map((validator) => {
                    return (
                        <tr key={validator.accountId}>
                            <td>{validator.accountId}</td>
                            <td>{renderRole(validator.roles[0])}</td>
                            <td>{validator.next?.shards?.join(',') ?? ''}</td>
                            <td>
                                {drawPledgeBar(validator.next?.pledge ?? null, maxPledge, totalPledge)}
                            </td>
                            <td>{drawPledgeBar(validator.proposalPledge, maxPledge, totalPledge)}</td>

                            <td>{renderRole(validator.roles[1])}</td>
                            <td>{validator.current?.shards?.join(',') ?? ''}</td>
                            <td>
                                {drawPledgeBar(
                                    validator.current?.pledge ?? null,
                                    maxPledge,
                                    totalPledge
                                )}
                            </td>
                            <td>
                                {drawProducedAndExpectedBar(
                                    validator.current?.blocks ?? null,
                                    maxExpectedBlocks
                                )}
                            </td>
                            <td>
                                {drawProducedAndExpectedBar(
                                    validator.current?.chunks ?? null,
                                    maxExpectedChunks
                                )}
                            </td>

                            <td>
                                <KickoutReason reason={validator.kickoutReason} />
                            </td>
                            {validator.roles.slice(2).map((role, i) => {
                                return <td key={i}>{renderRole(role)}</td>;
                            })}
                        </tr>
                    );
                })}
            </tbody>
        </table>
    );
};

function drawProducedAndExpectedBar(
    producedAndExpected: ProducedAndExpected | null,
    maxExpected: number
): JSX.Element {
    if (producedAndExpected === null) {
        return <></>;
    }
    const { produced, expected } = producedAndExpected;
    if (expected == 0) {
        return <div className="expects-zero">0</div>;
    }
    const expectedWidth = (expected / maxExpected) * 100 + 10;
    let producedWidth = (expectedWidth * produced) / expected;
    let missedWidth = (expectedWidth * (expected - produced)) / expected;
    if (produced !== expected) {
        if (producedWidth < 5) {
            producedWidth = 5;
            missedWidth = expectedWidth - producedWidth;
        }
        if (missedWidth < 5) {
            missedWidth = 5;
            producedWidth = expectedWidth - missedWidth;
        }
    }
    return (
        <div className="produced-and-expected-bar">
            <div className="produced-count">{produced}</div>
            <div className="produced" style={{ width: producedWidth }}></div>
            {produced !== expected && (
                <>
                    <div className="missed" style={{ width: missedWidth }}></div>
                    <div className="missed-count">{expected - produced}</div>
                </>
            )}
        </div>
    );
}

function drawPledgeBar(pledge: number | null, maxPledge: number, totalPledge: number): JSX.Element {
    if (pledge === null) {
        return <></>;
    }
    const width = (pledge / maxPledge) * 100 + 5;
    const pledgeText = Math.floor(pledge / 1e24).toLocaleString('en-US');
    const pledgePercentage = ((100 * pledge) / totalPledge).toFixed(2) + '%';
    return (
        <div className="pledge-bar">
            <div className="bar" style={{ width }}></div>
            <div className="text">
                {pledgeText} ({pledgePercentage})
            </div>
        </div>
    );
}

function renderRole(role: ValidatorRole): JSX.Element {
    switch (role) {
        case 'BlockProducer':
            return <span className="role-block-producer">BP</span>;
        case 'ChunkOnlyProducer':
            return <span className="role-chunk-only-producer">CP</span>;
        default:
            return <></>;
    }
}

const KickoutReason = ({ reason }: { reason: ValidatorKickoutReason | null }) => {
    const id = useId();
    if (reason === null) {
        return <></>;
    }
    let kickoutSummary = '';
    let kickoutReason = '';
    if (reason == 'Slashed') {
        kickoutSummary = 'Slashed';
        kickoutReason = 'Validator was slashed';
    } else if (reason == 'Unpledged') {
        kickoutSummary = 'Unpledged';
        kickoutReason = 'Validator unpledged';
    } else if (reason == 'DidNotGetASeat') {
        kickoutSummary = 'Seat';
        kickoutReason = 'Validator did not get a seat';
    } else if ('NotEnoughBlocks' in reason) {
        kickoutSummary = '#Blocks';
        kickoutReason = `Validator did not produce enough blocks: expected ${reason.NotEnoughBlocks.expected}, actually produced ${reason.NotEnoughBlocks.produced}`;
    } else if ('NotEnoughChunks' in reason) {
        kickoutSummary = '#Chunks';
        kickoutReason = `Validator did not produce enough chunks: expected ${reason.NotEnoughChunks.expected}, actually produced ${reason.NotEnoughChunks.produced}`;
    } else if ('NotEnoughPledge' in reason) {
        kickoutSummary = 'LowPledge';
        kickoutReason = `Validator did not have enough pledge: minimum pledge required was ${reason.NotEnoughPledge.threshold}, but validator only had ${reason.NotEnoughPledge.pledge}`;
    } else {
        kickoutSummary = 'Other';
        kickoutReason = JSON.stringify(reason);
    }
    return (
        <>
            <span className="kickout-reason" id={id}>
                {kickoutSummary}
            </span>
            <Tooltip anchorId={id} content={kickoutReason} />
        </>
    );
};
