import React, { useEffect, useState } from 'react';
import { useAuth } from '../../providers/useAuth';
import { useCollectionStore } from '../../stores/collectionStore';
import { Skeleton } from '../atoms/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../atoms/tabs';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../atoms/card';
import { Button } from '../atoms/button';
import { Modal } from '../atoms/modal';
import { CreateCollectionModal } from '../molecules/modals/CreateCollectionModal';
import { useToast } from '../../providers/toast';
import { GovernanceModel, Proposal, ProposalStatus } from 'declarations/clanopedia_backend/clanopedia_backend.did';
import { Principal } from '@dfinity/principal';
import { useNavigate } from 'react-router-dom';
import { cn } from '../../lib/utils';

export const formatGovernanceModel = (model: GovernanceModel): string => {
    if ('TokenBased' in model) return 'Token Based';
    if ('Multisig' in model) return 'Multisig';
    if ('SnsIntegrated' in model) return 'SNS Integrated';
    if ('Permissionless' in model) return 'Admin';
    return 'Unknown';
};

export const formatProposalStatus = (status: ProposalStatus): string => {
    if ('Active' in status) return 'Active';
    if ('Approved' in status) return 'Approved';
    if ('Rejected' in status) return 'Rejected';
    if ('Executed' in status) return 'Executed';
    if ('Expired' in status) return 'Expired';
    return 'Unknown';
};

export const Collections: React.FC = () => {
    const { ClanopediaActor, principal, isAuthenticated } = useAuth();
    const { toast } = useToast();
    const navigate = useNavigate();
    const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
    const {
        collections,
        isLoading,
        error,
        activeTab,
        setActiveTab,
        fetchCollections
    } = useCollectionStore();

    useEffect(() => {
        if (ClanopediaActor) {
            fetchCollections(ClanopediaActor, principal);
        }
    }, [ClanopediaActor, principal]);

    const handleCreateCollection = async (formData: {
        name: string;
        description: string;
        admins: Principal[];
        threshold: number;
        governance_model: GovernanceModel;
        governance_token?: Principal[];
        quorum_threshold: number;
        is_permissionless: boolean;
        sns_governance_canister?: string;
    }) => {
        if (!ClanopediaActor) return;
        console.log('Creating collection with form data:', {
            ...formData,
            admins: formData.admins.map(p => p.toString()),
            governance_token: formData.governance_token ? formData.governance_token.map(p => p.toString()) : []
        });

        try {
            const backendConfig: any = {
                threshold: formData.threshold || 0,
                name: formData.name,
                description: formData.description,
                admins: formData.admins.map(p => p.toString()),
                is_permissionless: formData.is_permissionless,
                governance_model: formData.governance_model,
                governance_token: formData.governance_token ? formData.governance_token.map(p => p.toString()) : [],
                quorum_threshold: formData.quorum_threshold,
                sns_governance_canister: formData.sns_governance_canister && formData.sns_governance_canister !== '' ? formData.sns_governance_canister : null,
            };
            const result = await ClanopediaActor.create_collection_endpoint(backendConfig);

            if ('Ok' in result) {
                toast({
                    title: "Collection created",
                    description: "Your collection has been created successfully",
                });
                // Refresh the collections list
                fetchCollections(ClanopediaActor, principal);
            } else {
                const errorObj = result.Err;
                console.error('Canister error:', errorObj);
                // Extract the error message from the error object
                let errorMessage = 'Failed to create collection';
                if ('BluebandError' in errorObj) {
                    errorMessage = errorObj.BluebandError;
                } else if ('InvalidInput' in errorObj) {
                    errorMessage = errorObj.InvalidInput;
                } else if ('NotAuthorized' in errorObj) {
                    errorMessage = 'You are not authorized to perform this action';
                } else if ('InvalidOperation' in errorObj) {
                    errorMessage = errorObj.InvalidOperation;
                }
                throw new Error(errorMessage);
            }
        } catch (error) {
            console.error('Error creating collection:', error);
            if (error instanceof Error) {
                throw error;
            } else {
                throw new Error('Failed to create collection: ' + JSON.stringify(error));
            }
        }
    };

    const handleViewProposals = (collectionId: string) => {
        navigate(`/collections/${collectionId}/proposals`);
    };

    const handleCardClick = (e: React.MouseEvent, collectionId: string) => {
        // Prevent navigation if clicking on the View Proposals button
        if ((e.target as HTMLElement).closest('button')) {
            return;
        }
        navigate(`/collections/${collectionId}`);
    };

    const filteredCollections = collections.filter(collection => {
        if (activeTab === 'all') return true;
        if (!isAuthenticated) return false;
        return collection.isOwner || collection.isAdmin;
    });

    if (error) {
        return (
            <div className="p-4 text-red-500">
                Error loading collections: {error}
            </div>
        );
    }

    return (
        <div className="container mx-auto p-4">
            <div className="flex justify-between items-center mb-6">
                <h1 className="text-2xl font-bold">Collections</h1>
                {isAuthenticated && (
                    <Button
                        onClick={() => setIsCreateModalOpen(true)}
                        className="bg-primary hover:bg-primary/90"
                    >
                        Create Collection
                    </Button>
                )}
            </div>

            <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as 'all' | 'forYou')}>
                <TabsList className="mb-4">
                    <TabsTrigger value="all">All Collections</TabsTrigger>
                    {isAuthenticated && (
                        <TabsTrigger value="forYou">For You</TabsTrigger>
                    )}
                </TabsList>

                <TabsContent value={activeTab}>
                    {isLoading ? (
                        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                            {[...Array(6)].map((_, i) => (
                                <Card key={i}>
                                    <CardHeader>
                                        <Skeleton className="h-6 w-3/4" />
                                        <Skeleton className="h-4 w-1/2" />
                                    </CardHeader>
                                    <CardContent>
                                        <Skeleton className="h-20 w-full" />
                                    </CardContent>
                                </Card>
                            ))}
                        </div>
                    ) : filteredCollections.length === 0 ? (
                        <div className="text-center p-8 text-gray-500">
                            {isAuthenticated ? (
                                <div className="space-y-4">
                                    <p>No collections found</p>
                                    <Button
                                        onClick={() => setIsCreateModalOpen(true)}
                                        variant="outline"
                                    >
                                        Create your first collection
                                    </Button>
                                </div>
                            ) : (
                                "No collections found"
                            )}
                        </div>
                    ) : (
                        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                            {filteredCollections.map((collection) => (
                                <Card 
                                    key={collection.id}
                                    className={cn(
                                        "transition-all duration-200 hover:shadow-lg cursor-pointer",
                                        "hover:border-primary/50"
                                    )}
                                    onClick={(e) => handleCardClick(e, collection.id)}
                                >
                                    <CardHeader>
                                        <CardTitle className="flex items-center justify-between">
                                            {collection.name}
                                            {(collection.isOwner || collection.isAdmin) && (
                                                <span className="text-xs bg-blue-100 text-blue-800 px-2 py-1 rounded">
                                                    {collection.isOwner ? 'Owner' : 'Admin'}
                                                </span>
                                            )}
                                        </CardTitle>
                                        <CardDescription>
                                            {collection.description}
                                        </CardDescription>
                                    </CardHeader>
                                    <CardContent>
                                        <div className="space-y-2">
                                            <p className="text-sm text-gray-500">
                                                Created by: {collection.creator.toString()}
                                            </p>
                                            <p className="text-sm text-gray-500">
                                                Proposals: {Object.keys(collection.proposals).length}
                                            </p>
                                            <p className="text-sm text-gray-500">
                                                Governance Model: {formatGovernanceModel(collection.governance_model)}
                                            </p>
                                            {collection.governance_model && (
                                                <p className="text-sm text-gray-500">
                                                    {(() => {
                                                        if ('TokenBased' in collection.governance_model && collection.governance_token) {
                                                            return <>Token: <span className="font-mono">{collection.governance_token.toString()}</span> • Quorum: {collection.quorum_threshold}%</>;
                                                        }
                                                        if ('Multisig' in collection.governance_model) {
                                                            return <>Threshold: {collection.threshold}</>;
                                                        }
                                                        if ('SnsIntegrated' in collection.governance_model && collection.sns_governance_canister) {
                                                            return <>SNS Canister: <span className="font-mono">{collection.sns_governance_canister.toString()}</span></>;
                                                        }
                                                        return null;
                                                    })()}
                                                </p>
                                            )}
                                            {(collection.isOwner || collection.isAdmin) && (
                                                <div className="flex space-x-2 mt-4 justify-end">
                                                    <Button
                                                        variant="outline"
                                                        size="sm"
                                                        onClick={(e) => {
                                                            e.stopPropagation();
                                                            handleViewProposals(collection.id);
                                                        }}
                                                    >
                                                        View Proposals
                                                    </Button>
                                                </div>
                                            )}
                                        </div>
                                    </CardContent>
                                </Card>
                            ))}
                        </div>
                    )}
                </TabsContent>
            </Tabs>

            <Modal isOpen={isCreateModalOpen} onClose={() => setIsCreateModalOpen(false)}>
                <CreateCollectionModal
                    onClose={() => setIsCreateModalOpen(false)}
                    onSubmit={handleCreateCollection}
                />
            </Modal>
        </div>
    );
}; 