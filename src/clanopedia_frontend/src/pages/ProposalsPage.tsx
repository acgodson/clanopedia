import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useAuth } from '../providers/useAuth';
import { useToast } from '../providers/toast';
import { Card, CardContent, CardHeader, CardTitle } from '../components/atoms/card';
import { Button } from '../components/atoms/button';
import { Loader2, CheckCircle2, XCircle, Clock, AlertCircle } from 'lucide-react';
import { useCollectionStore } from '../stores/collectionStore';
import { Principal } from '@dfinity/principal';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '../components/atoms/dialog';
import { Input } from '../components/atoms/input';

interface Proposal {
    id: string;
    description: string;
    status: {
        Active?: null;
        Approved?: null;
        Rejected?: null;
        Executed?: null;
        Expired?: null;
    };
    created_at: bigint;
    expires_at: bigint;
    creator: string;
    proposal_type: {
        BatchEmbed?: { document_ids: string[] };
        EmbedDocument?: { documents: string[] };
        [key: string]: any;
    };
    votes?: { voter: string; vote: string }[];
    sns_proposal_id?: number;
}

function isNonEmptyString(val: unknown): val is string {
    return typeof val === 'string' && val.trim() !== '';
}

export function ProposalsPage() {
    const { collectionId } = useParams<{ collectionId: string }>();
    const navigate = useNavigate();
    const { ClanopediaActor } = useAuth();
    const { toast } = useToast();
    const [proposals, setProposals] = useState<Proposal[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [executingProposals, setExecutingProposals] = useState<Set<string>>(new Set());
    const { collections } = useCollectionStore();
    const [currentCollection, setCurrentCollection] = useState<any>(null);
    const [voteDialogOpen, setVoteDialogOpen] = useState<string | null>(null);
    const [snsLinkDialogOpen, setSnsLinkDialogOpen] = useState<string | null>(null);
    const [snsProposalIdInput, setSnsProposalIdInput] = useState('');
    const [voting, setVoting] = useState(false);
    const [linking, setLinking] = useState(false);
    const { principal } = useAuth();
    const [userPrincipal, setUserPrincipal] = useState<string | null>(null);
    const [syncingProposalId, setSyncingProposalId] = useState<string | null>(null);

    useEffect(() => {
        const fetchProposals = async () => {
            if (!ClanopediaActor || !collectionId) return;

            try {
                const result = await ClanopediaActor.get_proposals_endpoint(collectionId);
                if ('Ok' in result) {
                    // Normalize votes to array
                    const normalizeVotes = (proposal: any) => {
                        if (proposal.votes && !Array.isArray(proposal.votes)) {
                            return Object.entries(proposal.votes).map(([voter, vote]) => {
                                if (vote && typeof vote === 'object') {
                                    if (vote.hasOwnProperty('Yes')) return { voter: voter.toString(), vote: 'Yes' };
                                    if (vote.hasOwnProperty('No')) return { voter: voter.toString(), vote: 'No' };
                                    if (vote.hasOwnProperty('Abstain')) return { voter: voter.toString(), vote: 'Abstain' };
                                }
                                // fallback if vote is null or not an object
                                return { voter: voter.toString(), vote: vote ? vote.toString() : 'Abstain' };
                            });
                        }
                        return proposal.votes || [];
                    };
                    const normalizedProposals = result.Ok.map((proposal: any) => ({
                        ...proposal,
                        votes: normalizeVotes(proposal)
                    }));
                    setProposals(normalizedProposals);
                } else {
                    console.error('Failed to fetch proposals:', result.Err);
                    toast({
                        title: "Error",
                        description: "Failed to fetch proposals",
                        variant: "destructive",
                    });
                }
            } catch (error) {
                console.error('Error fetching proposals:', error);
                toast({
                    title: "Error",
                    description: "Failed to fetch proposals",
                    variant: "destructive",
                });
            } finally {
                setIsLoading(false);
            }
        };

        fetchProposals();
    }, [ClanopediaActor, collectionId, toast]);

    useEffect(() => {
        if (collections && collectionId) {
            const found = collections.find((c: any) => c.id === collectionId);
            setCurrentCollection(found);
        }
        setUserPrincipal(principal ? principal.toString() : null);
    }, [collections, collectionId, principal]);

    const handleExecuteProposal = async (proposalId: string) => {
        if (!ClanopediaActor || !collectionId) return;

        // Add proposal to executing set
        setExecutingProposals(prev => new Set(prev).add(proposalId));

        try {
            const result = await ClanopediaActor.execute_proposal_endpoint(collectionId, proposalId);
            if ('Ok' in result) {
                toast({
                    title: "Success",
                    description: "Proposal executed successfully. Documents are being embedded.",
                });
                // Refresh proposals
                const updatedResult = await ClanopediaActor.get_proposals_endpoint(collectionId);
                if ('Ok' in updatedResult) {
                    setProposals(updatedResult.Ok);
                }
            } else {
                // Handle specific error types from the backend
                const error = result.Err;
                let errorMessage = 'Failed to execute proposal';
                
                if (typeof error === 'object') {
                    if ('NotAuthorized' in error) {
                        errorMessage = 'You are not authorized to execute this proposal';
                    } else if ('InvalidOperation' in error) {
                        errorMessage = error.InvalidOperation;
                    } else if ('InvalidInput' in error) {
                        errorMessage = error.InvalidInput;
                    } else if ('BluebandError' in error) {
                        errorMessage = `Blueband error: ${error.BluebandError}`;
                    } else {
                        errorMessage = `Error: ${JSON.stringify(error)}`;
                    }
                } else if (typeof error === 'string') {
                    errorMessage = error;
                }

                toast({
                    title: "Error",
                    description: errorMessage,
                    variant: "destructive",
                });
            }
        } catch (error) {
            console.error('Error executing proposal:', error);
            toast({
                title: "Error",
                description: error instanceof Error ? error.message : "Failed to execute proposal",
                variant: "destructive",
            });
        } finally {
            // Remove proposal from executing set
            setExecutingProposals(prev => {
                const newSet = new Set(prev);
                newSet.delete(proposalId);
                return newSet;
            });
        }
    };

    const formatDate = (timestamp: bigint) => {
        return new Date(Number(timestamp) / 1_000_000).toLocaleString();
    };

    const getStatusText = (status: Proposal['status']): string => {
        if ('Active' in status) return 'Active';
        if ('Approved' in status) return 'Approved';
        if ('Rejected' in status) return 'Rejected';
        if ('Executed' in status) return 'Executed';
        if ('Expired' in status) return 'Expired';
        return 'Unknown';
    };

    const getStatusIcon = (status: Proposal['status']) => {
        switch (getStatusText(status)) {
            case 'Active':
                return <Clock className="h-4 w-4 text-blue-500" />;
            case 'Approved':
                return <CheckCircle2 className="h-4 w-4 text-green-500" />;
            case 'Rejected':
                return <XCircle className="h-4 w-4 text-red-500" />;
            case 'Executed':
                return <CheckCircle2 className="h-4 w-4 text-green-500" />;
            case 'Expired':
                return <AlertCircle className="h-4 w-4 text-yellow-500" />;
            default:
                return null;
        }
    };

    const getProposalTypeDescription = (proposal: Proposal): string => {
        const proposalType = proposal.proposal_type;
        if ('BatchEmbed' in proposalType && proposalType.BatchEmbed) {
            const docCount = proposalType.BatchEmbed.document_ids.length;
            return `Batch Embed ${docCount} document${docCount > 1 ? 's' : ''}`;
        }
        if ('EmbedDocument' in proposalType && proposalType.EmbedDocument) {
            const docCount = proposalType.EmbedDocument.documents.length;
            return `Embed ${docCount} document${docCount > 1 ? 's' : ''}`;
        }
        return 'Unknown proposal type';
    };

    // Helper: get governance model
    const getGovernanceModel = () => {
        if (!currentCollection) return 'Unknown';
        if ('TokenBased' in currentCollection.governance_model) return 'TokenBased';
        if ('Multisig' in currentCollection.governance_model) return 'Multisig';
        if ('SnsIntegrated' in currentCollection.governance_model) return 'SnsIntegrated';
        if ('Permissionless' in currentCollection.governance_model) return 'Permissionless';
        return 'Unknown';
    };

    // Helper: get user's vote for a proposal
    const getUserVote = (proposal: any) => {
        if (!userPrincipal || !proposal.votes) return null;
        const vote = proposal.votes.find((v: any) => v.voter === userPrincipal);
        return vote ? vote.vote : null;
    };

    // Voting handler
    const handleVote = async (proposalId: string, vote: 'Yes' | 'No') => {
        if (!ClanopediaActor || !collectionId) return;
        setVoting(true);
        try {
            const result = await ClanopediaActor.vote_on_proposal_endpoint(collectionId, proposalId, { [vote]: null });
            if ('Ok' in result) {
                toast({ title: 'Vote submitted', description: `You voted ${vote}` });
                // Refresh proposals
                const updatedResult = await ClanopediaActor.get_proposals_endpoint(collectionId);
                if ('Ok' in updatedResult) setProposals(updatedResult.Ok);
            } else {
                const error = result.Err;
                let errorMessage = 'Failed to vote';
                if (typeof error === 'object') {
                    if ('NotAuthorized' in error) {
                        errorMessage = 'You are not authorized to vote on this proposal';
                    } else if ('InvalidOperation' in error) {
                        errorMessage = error.InvalidOperation;
                    } else if ('InvalidInput' in error) {
                        errorMessage = error.InvalidInput;
                    } else if ('BluebandError' in error) {
                        errorMessage = `Blueband error: ${error.BluebandError}`;
                    } else {
                        errorMessage = `Error: ${JSON.stringify(error)}`;
                    }
                } else if (typeof error === 'string') {
                    errorMessage = error;
                }
                toast({ title: 'Error', description: errorMessage, variant: 'destructive' });
            }
        } catch (e) {
            toast({ title: 'Error', description: 'Failed to vote', variant: 'destructive' });
        } finally {
            setVoting(false);
            setVoteDialogOpen(null);
        }
    };

    // SNS proposal linking handler
    const handleLinkSnsProposal = async (proposalId: string) => {
        if (!ClanopediaActor || !collectionId) return;
        setLinking(true);
        try {
            const snsProposalId = parseInt(snsProposalIdInput, 10);
            if (isNaN(snsProposalId)) throw new Error('Invalid SNS proposal ID');
            const result = await ClanopediaActor.link_sns_proposal_id_endpoint(collectionId, proposalId, snsProposalId);
            if ('Ok' in result) {
                toast({ title: 'SNS Proposal Linked', description: 'SNS proposal ID linked successfully.' });
                // Refresh proposals
                const updatedResult = await ClanopediaActor.get_proposals_endpoint(collectionId);
                if ('Ok' in updatedResult) setProposals(updatedResult.Ok);
            } else {
                toast({ title: 'Error', description: 'Failed to link SNS proposal', variant: 'destructive' });
            }
        } catch (e) {
            toast({ title: 'Error', description: 'Failed to link SNS proposal', variant: 'destructive' });
        } finally {
            setLinking(false);
            setSnsLinkDialogOpen(null);
            setSnsProposalIdInput('');
        }
    };

    const handleSyncSnsStatus = async (proposalId: string) => {
        if (!ClanopediaActor || !collectionId) return;
        setSyncingProposalId(proposalId);
        try {
            const result = await ClanopediaActor.sync_sns_proposal_status_and_update_endpoint(collectionId, proposalId);
            if ('Ok' in result) {
                toast({ title: 'SNS Status Synced', description: 'Proposal status updated from SNS.' });
                // Refresh proposals
                const updatedResult = await ClanopediaActor.get_proposals_endpoint(collectionId);
                if ('Ok' in updatedResult) setProposals(updatedResult.Ok);
            } else {
                toast({ title: 'Error', description: 'Failed to sync SNS proposal status', variant: 'destructive' });
            }
        } catch (e) {
            toast({ title: 'Error', description: 'Failed to sync SNS proposal status', variant: 'destructive' });
        } finally {
            setSyncingProposalId(null);
        }
    };

    if (isLoading) {
        return (
            <div className="flex items-center justify-center h-[calc(100vh-4rem-64px)]">
                <Loader2 className="h-8 w-8 animate-spin" />
            </div>
        );
    }

    return (
        <div className="space-y-6">
            <div className="flex items-center justify-between">
                <h1 className="text-2xl font-bold">Proposals</h1>
                <Button variant="outline" onClick={() => navigate(`/collections/${collectionId}`)}>
                    Back to Collection
                </Button>
            </div>

            <div className="grid gap-4">
                {proposals.length === 0 ? (
                    <Card>
                        <CardContent className="pt-6">
                            <p className="text-center text-muted-foreground">No proposals found</p>
                        </CardContent>
                    </Card>
                ) : (
                    proposals.map((proposal) => (
                        <Card key={proposal.id}>
                            <CardHeader>
                                <CardTitle className="text-lg">{proposal.description}</CardTitle>
                                <div className="flex items-center space-x-2 text-sm text-muted-foreground">
                                    <div className="flex items-center gap-1">
                                        {getStatusIcon(proposal.status)}
                                        <span>Status: {getStatusText(proposal.status)}</span>
                                    </div>
                                    <span>•</span>
                                    <span>{getProposalTypeDescription(proposal)}</span>
                                    <span>•</span>
                                    <span>Created: {formatDate(proposal.created_at)}</span>
                                    <span>•</span>
                                    <span>Expires: {formatDate(proposal.expires_at)}</span>
                                </div>
                            </CardHeader>
                            <CardContent>
                                <div className="flex justify-end space-x-2">
                                    {(() => {
                                        const model = getGovernanceModel();
                                        const userVote = getUserVote(proposal);
                                        const status = getStatusText(proposal.status);
                                        const isAdmin = currentCollection && (
                                            (currentCollection.admins && currentCollection.admins.includes(userPrincipal)) ||
                                            (currentCollection.creator && currentCollection.creator.toString && currentCollection.creator.toString() === userPrincipal) ||
                                            currentCollection.isOwner // if present from store
                                        );
                                        if (model === 'TokenBased' && status === 'Active') {
                                            return (
                                                <>
                                                    {userVote ? (
                                                        <span className="text-sm">You voted: {userVote}</span>
                                                    ) : (
                                                        <Button onClick={() => setVoteDialogOpen(proposal.id)} disabled={voting}>Vote Yes</Button>
                                                    )}
                                                    <Dialog open={voteDialogOpen === proposal.id} onOpenChange={() => setVoteDialogOpen(null)}>
                                                        <DialogContent>
                                                            <DialogHeader><DialogTitle>Vote on Proposal</DialogTitle></DialogHeader>
                                                            <div>Do you want to vote <b>Yes</b> on this proposal?</div>
                                                            <DialogFooter>
                                                                <Button onClick={() => handleVote(proposal.id, 'Yes')} disabled={voting}>Yes</Button>
                                                                <Button variant="outline" onClick={() => setVoteDialogOpen(null)}>Cancel</Button>
                                                            </DialogFooter>
                                                        </DialogContent>
                                                    </Dialog>
                                                </>
                                            );
                                        }
                                        if (model === 'TokenBased' && status === 'Approved' && isAdmin) {
                                            return (
                                                <Button
                                                    onClick={() => handleExecuteProposal(proposal.id)}
                                                    disabled={executingProposals.has(proposal.id)}
                                                >
                                                    {executingProposals.has(proposal.id) ? 'Executing...' : 'Execute Proposal'}
                                                </Button>
                                            );
                                        }
                                        if (model === 'TokenBased' && status === 'Executed') {
                                            return <span className="text-sm text-green-600">Proposal executed</span>;
                                        }
                                        if (model === 'Multisig' && status === 'Active' && isAdmin) {
                                            return (
                                                <>
                                                    {userVote ? (
                                                        <span className="text-sm">You approved this proposal</span>
                                                    ) : (
                                                        <Button onClick={() => setVoteDialogOpen(proposal.id)} disabled={voting}>Approve</Button>
                                                    )}
                                                    <Dialog open={voteDialogOpen === proposal.id} onOpenChange={() => setVoteDialogOpen(null)}>
                                                        <DialogContent>
                                                            <DialogHeader><DialogTitle>Approve Proposal</DialogTitle></DialogHeader>
                                                            <div>Do you want to approve this proposal?</div>
                                                            <DialogFooter>
                                                                <Button onClick={() => handleVote(proposal.id, 'Yes')} disabled={voting}>Approve</Button>
                                                                <Button variant="outline" onClick={() => setVoteDialogOpen(null)}>Cancel</Button>
                                                            </DialogFooter>
                                                        </DialogContent>
                                                    </Dialog>
                                                </>
                                            );
                                        }
                                        if (model === 'Multisig' && status === 'Approved' && isAdmin) {
                                            return (
                                                <Button
                                                    onClick={() => handleExecuteProposal(proposal.id)}
                                                    disabled={executingProposals.has(proposal.id)}
                                                >
                                                    {executingProposals.has(proposal.id) ? 'Executing...' : 'Execute Proposal'}
                                                </Button>
                                            );
                                        }
                                        if (model === 'Multisig' && status === 'Executed') {
                                            return <span className="text-sm text-green-600">Proposal executed</span>;
                                        }
                                        if (model === 'SnsIntegrated') {
                                            // Show SNS proposal ID and link dialog if not linked
                                            const hasSnsProposalId = proposal.sns_proposal_id !== undefined && proposal.sns_proposal_id !== null && (
                                                (typeof proposal.sns_proposal_id === 'number' && proposal.sns_proposal_id > 0) ||
                                                isNonEmptyString(proposal.sns_proposal_id)
                                            );
                                            return (
                                                <>
                                                    {hasSnsProposalId ? (
                                                        <div className="flex items-center gap-2">
                                                            <span className="text-sm">SNS Proposal ID: {proposal.sns_proposal_id}</span>
                                                            <Button
                                                                variant="outline"
                                                                size="sm"
                                                                onClick={() => handleSyncSnsStatus(proposal.id)}
                                                                disabled={syncingProposalId === proposal.id}
                                                            >
                                                                {syncingProposalId === proposal.id ? 'Syncing...' : 'Sync Status'}
                                                            </Button>
                                                        </div>
                                                    ) : (
                                                        <Button onClick={() => setSnsLinkDialogOpen(proposal.id)} disabled={linking}>Link SNS Proposal</Button>
                                                    )}
                                                    <Dialog open={snsLinkDialogOpen === proposal.id} onOpenChange={() => setSnsLinkDialogOpen(null)}>
                                                        <DialogContent>
                                                            <DialogHeader><DialogTitle>Link SNS Proposal</DialogTitle></DialogHeader>
                                                            <Input placeholder="SNS Proposal ID" value={snsProposalIdInput} onChange={e => setSnsProposalIdInput(e.target.value)} />
                                                            <DialogFooter>
                                                                <Button onClick={() => handleLinkSnsProposal(proposal.id)} disabled={linking}>Link</Button>
                                                                <Button variant="outline" onClick={() => setSnsLinkDialogOpen(null)}>Cancel</Button>
                                                            </DialogFooter>
                                                        </DialogContent>
                                                    </Dialog>
                                                </>
                                            );
                                        }
                                        if (model === 'Permissionless' && status === 'Approved' && isAdmin) {
                                            return (
                                                <Button
                                                    onClick={() => handleExecuteProposal(proposal.id)}
                                                    disabled={executingProposals.has(proposal.id)}
                                                >
                                                    {executingProposals.has(proposal.id) ? 'Executing...' : 'Execute Proposal'}
                                                </Button>
                                            );
                                        }
                                        if (model === 'Permissionless' && status === 'Executed') {
                                            return <span className="text-sm text-green-600">Proposal executed</span>;
                                        }
                                        return null;
                                    })()}
                                </div>
                            </CardContent>
                        </Card>
                    ))
                )}
            </div>
        </div>
    );
} 