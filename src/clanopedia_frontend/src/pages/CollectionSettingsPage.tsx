import React, { useState, useEffect } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { Button } from '../components/atoms/button';
import { Card, CardContent, CardHeader, CardTitle } from '../components/atoms/card';
import { Input } from '../components/atoms/input';
import { Label } from '../components/atoms/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../components/atoms/select';
import { useAuth } from '../providers/useAuth';
import { useCollectionStore } from '../stores/collectionStore';
import { formatGovernanceModel } from '../components/organisms/Collections';
import { Modal } from '../components/atoms/modal';
import { useToast } from '../providers/toast';
import { GovernanceModel } from 'declarations/clanopedia_backend/clanopedia_backend.did';

interface CollectionSettings {
    name: string;
    description: string;
    threshold: number;
    quorum_threshold: number;
    is_permissionless: boolean;
    governance_model: GovernanceModel;
}

export function CollectionSettingsPage() {
    const { toast } = useToast();
    const { collectionId } = useParams<{ collectionId: string }>();
    const navigate = useNavigate();
    const { ClanopediaActor, principal, isAuthenticated } = useAuth();
    const { collections, isLoading, fetchCollections } = useCollectionStore();
    const [isEditing, setIsEditing] = useState(false);
    const [isDeleting, setIsDeleting] = useState(false);
    const [isDeleteModalOpen, setIsDeleteModalOpen] = useState(false);
    const [settings, setSettings] = useState<CollectionSettings>({
        name: '',
        description: '',
        threshold: 1,
        quorum_threshold: 50,
        is_permissionless: false,
        governance_model: { Multisig: null }
    });
    const [isAdmin, setIsAdmin] = useState(false);
    const [isCheckingAdmin, setIsCheckingAdmin] = useState(true);
    const [isSaving, setIsSaving] = useState(false);

    const currentCollection = collections.find(c => c.id === collectionId);

    // Fetch collections and check admin status
    useEffect(() => {
        const initialize = async () => {
            if (ClanopediaActor && collectionId) {
                setIsCheckingAdmin(true);
                await fetchCollections(ClanopediaActor, principal);
                await checkAdminStatus();
                setIsCheckingAdmin(false);
            }
        };
        initialize();
    }, [ClanopediaActor, principal, collectionId]);

    // Update settings when collection data is loaded
    useEffect(() => {
        if (currentCollection) {
            setSettings({
                name: currentCollection.name,
                description: currentCollection.description,
                threshold: currentCollection.threshold,
                quorum_threshold: currentCollection.quorum_threshold,
                is_permissionless: currentCollection.is_permissionless,
                governance_model: currentCollection.governance_model
            });
        }
    }, [currentCollection]);

    const checkAdminStatus = async () => {
        if (ClanopediaActor && collectionId && principal) {
            try {
                const adminStatus = await ClanopediaActor.is_admin_check(collectionId, principal);
                setIsAdmin(adminStatus);
            } catch (error) {
                console.error('Error checking admin status:', error);
                setIsAdmin(false);
            }
        }
    };

    // Redirect if not admin or collection not found, but only after we've checked
    useEffect(() => {
        if (!isLoading && !isCheckingAdmin && (!currentCollection || !isAdmin)) {
            navigate(`/collections/${collectionId}`);
        }
    }, [isLoading, isCheckingAdmin, currentCollection, isAdmin, collectionId, navigate]);

    const handleSave = async () => {
        if (!ClanopediaActor || !collectionId || !currentCollection) return;

        setIsSaving(true);
        try {
            // Create config object matching backend type
            const config = {
                name: settings.name,
                description: settings.description,
                threshold: settings.threshold,
                quorum_threshold: settings.quorum_threshold,
                is_permissionless: settings.is_permissionless,
                governance_model: settings.governance_model,
                admins: currentCollection.admins, // Keep existing admins
                governance_token: currentCollection.governance_token // Keep existing token
            };

            await ClanopediaActor.update_collection(collectionId, config);
            toast({
                title: "Settings updated",
                description: "Collection settings have been saved successfully."
            });
            setIsEditing(false);
            fetchCollections(ClanopediaActor, principal);
        } catch (error) {
            toast({
                title: "Error",
                description: "Failed to update settings. Please try again.",
                variant: "destructive"
            });
        } finally {
            setIsSaving(false);
        }
    };

    const handleDelete = async () => {
        if (!ClanopediaActor || !collectionId) return;

        setIsDeleting(true);
        try {
            await ClanopediaActor.delete_collection_endpoint(collectionId);
            toast({
                title: "Collection deleted",
                description: "The collection has been deleted successfully."
            });
            navigate('/collections');
        } catch (error) {
            toast({
                title: "Error",
                description: "Failed to delete collection. Please try again.",
                variant: "destructive"
            });
            setIsDeleteModalOpen(false);
        } finally {
            setIsDeleting(false);
        }
    };

    if (isLoading || isCheckingAdmin || !currentCollection) {
        return (
            <div className="min-h-screen bg-background p-8">
                <div className="mx-auto max-w-2xl space-y-8">
                    <div className="text-center py-10">
                        <p>Loading collection settings...</p>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="min-h-screen bg-background p-8">
            <div className="mx-auto max-w-2xl space-y-8">
                <header className="flex items-center justify-between mb-8">
                    <h1 className="text-3xl font-extrabold tracking-tight text-foreground">
                        <Link to={`/collections/${collectionId}`} className="text-muted-foreground hover:text-foreground">←</Link> Collection Settings
                    </h1>
                    <div className="flex items-center space-x-4">
                        {!isEditing ? (
                            <Button onClick={() => setIsEditing(true)} variant="outline">
                                ✏️ Edit Settings
                            </Button>
                        ) : (
                            <div className="flex space-x-2">
                                <Button onClick={() => {
                                    setIsEditing(false);
                                    setSettings({
                                        name: currentCollection.name,
                                        description: currentCollection.description,
                                        threshold: currentCollection.threshold,
                                        quorum_threshold: currentCollection.quorum_threshold,
                                        is_permissionless: currentCollection.is_permissionless,
                                        governance_model: currentCollection.governance_model
                                    });
                                }} variant="outline">
                                    Cancel
                                </Button>
                                <Button onClick={handleSave} disabled={isSaving}>
                                    {isSaving ? 'Saving...' : 'Save Changes'}
                                </Button>
                            </div>
                        )}
                    </div>
                </header>

                <Card>
                    <CardHeader>
                        <CardTitle>Basic Settings</CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-4">
                        <div className="space-y-2">
                            <Label htmlFor="name">Collection Name</Label>
                            <Input
                                id="name"
                                value={settings.name}
                                onChange={(e) => setSettings({ ...settings, name: e.target.value })}
                                disabled={!isEditing}
                            />
                        </div>
                        <div className="space-y-2">
                            <Label htmlFor="description">Description</Label>
                            <Input
                                id="description"
                                value={settings.description}
                                onChange={(e) => setSettings({ ...settings, description: e.target.value })}
                                disabled={!isEditing}
                            />
                        </div>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle>Governance Settings</CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-4">
                        <div className="space-y-2">
                            <Label htmlFor="governance_model">Governance Model</Label>
                            <Select
                                value={formatGovernanceModel(settings.governance_model)}
                                onValueChange={(value) => {
                                    let model: GovernanceModel;
                                    switch (value) {
                                        case 'Multisig':
                                            model = { Multisig: null };
                                            break;
                                        case 'Token Based':
                                            model = { TokenBased: null };
                                            break;
                                        case 'SNS Integrated':
                                            model = { SnsIntegrated: null };
                                            break;
                                        case 'Permissionless':
                                            model = { Permissionless: null };
                                            break;
                                        default:
                                            model = { Multisig: null };
                                    }
                                    setSettings({ ...settings, governance_model: model });
                                }}
                                disabled={!isEditing}
                            >
                                <SelectTrigger>
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="Multisig">Multisig</SelectItem>
                                    <SelectItem value="Token Based">Token Based</SelectItem>
                                    <SelectItem value="SNS Integrated">SNS Integrated</SelectItem>
                                    <SelectItem value="Permissionless">Permissionless</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>
                        <div className="space-y-2">
                            <Label htmlFor="threshold">Approval Threshold</Label>
                            <Input
                                id="threshold"
                                type="number"
                                min="1"
                                value={settings.threshold}
                                onChange={(e) => setSettings({ ...settings, threshold: parseInt(e.target.value) })}
                                disabled={!isEditing}
                            />
                        </div>
                        <div className="space-y-2">
                            <Label htmlFor="quorum">Quorum Threshold (%)</Label>
                            <Input
                                id="quorum"
                                type="number"
                                min="1"
                                max="100"
                                value={settings.quorum_threshold}
                                onChange={(e) => setSettings({ ...settings, quorum_threshold: parseInt(e.target.value) })}
                                disabled={!isEditing}
                            />
                        </div>
                        <div className="flex items-center space-x-2">
                            <input
                                type="checkbox"
                                id="permissionless"
                                checked={settings.is_permissionless}
                                onChange={(e) => setSettings({ ...settings, is_permissionless: e.target.checked })}
                                disabled={!isEditing}
                                className="h-4 w-4 rounded border-gray-300"
                            />
                            <Label htmlFor="permissionless">Permissionless Collection</Label>
                        </div>
                    </CardContent>
                </Card>

                <Card className="border-destructive">
                    <CardHeader>
                        <CardTitle className="text-destructive">Danger Zone</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <Button
                            variant="destructive"
                            onClick={() => setIsDeleteModalOpen(true)}
                            disabled={isDeleting}
                        >
                            {isDeleting ? 'Deleting...' : 'Delete Collection'}
                        </Button>
                    </CardContent>
                </Card>

                <Modal isOpen={isDeleteModalOpen} onClose={() => setIsDeleteModalOpen(false)}>
                    <div className="p-6 space-y-4">
                        <h2 className="text-lg font-semibold">Delete Collection</h2>
                        <p className="text-muted-foreground">
                            Are you sure you want to delete this collection? This action cannot be undone.
                        </p>
                        <div className="flex justify-end space-x-2">
                            <Button variant="outline" onClick={() => setIsDeleteModalOpen(false)}>
                                Cancel
                            </Button>
                            <Button variant="destructive" onClick={handleDelete} disabled={isDeleting}>
                                {isDeleting ? 'Deleting...' : 'Delete Collection'}
                            </Button>
                        </div>
                    </div>
                </Modal>
            </div>
        </div>
    );
} 