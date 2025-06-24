import React, { useState, useEffect } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { Button } from '../components/atoms/button';
import { Card, CardContent, CardHeader, CardTitle } from '../components/atoms/card';
import { DocumentReader } from '../components/organisms/DocumentReader';
import { useQueryStore } from '../store/queryStore';
import { useCollectionStore } from '../stores/collectionStore';
import { useAuth } from '../providers/useAuth';
import { formatGovernanceModel } from '../components/organisms/Collections';
import { Skeleton } from '../components/atoms/skeleton';
import { DocumentAdder } from '../components/organisms/DocumentAdder';
import { Modal } from '../components/atoms/modal';
import { useToast } from '../providers/toast';
import { Input } from '../components/atoms/input';
import { Loader2 } from 'lucide-react';

interface QueryMessage {
    type: 'user' | 'system';
    text: string;
    timestamp: number;
    timeTaken?: number;
}

interface CollectionMetrics {
    document_count: bigint;
    search_count: bigint;
}

interface SearchResult {
    id: string;
    score: number;
    title: string;
    date: string;
    author: string;
    sourceLink: string | null;
    text: string;
}

export function CollectionPage() {
    const { collectionId } = useParams<{ collectionId: string }>();
    const navigate = useNavigate();
    const { ClanopediaActor, BluebandActor, principal, isAuthenticated } = useAuth();
    const { collections, isLoading, fetchCollections } = useCollectionStore();
    const [currentView, setCurrentView] = useState<'search' | 'reader'>('search');
    const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);
    const [searchTerm, setSearchTerm] = useState<string>('');
    const [queryMessages, setQueryMessages] = useState<QueryMessage[]>([]);
    const [isQuerying, setIsQuerying] = useState(false);
    const [isAddDocumentModalOpen, setIsAddDocumentModalOpen] = useState(false);
    const [isAdmin, setIsAdmin] = useState(false);
    const [metrics, setMetrics] = useState<CollectionMetrics | null>(null);

    const {
        addQuery,
        getCurrentQuery,
        getPreviousQuery,
        getNextQuery,
        hasPreviousQuery,
        hasNextQuery,
        setCurrentQueryIndex,
        currentQueryIndex,
        resetQueries,
    } = useQueryStore();

    const { toast } = useToast();

    // Fetch collections and find the current one
    useEffect(() => {
        if (ClanopediaActor && collectionId) {
            fetchCollections(ClanopediaActor, principal);
        }
    }, [ClanopediaActor, principal, collectionId]);

    const currentCollection = collections.find(c => c.id === collectionId);

    // Redirect if collection not found
    useEffect(() => {
        if (!isLoading && !currentCollection && collectionId) {
            navigate('/');
        }
    }, [isLoading, currentCollection, collectionId, navigate]);

    // Update messages when current query changes
    useEffect(() => {
        const currentQuery = getCurrentQuery();
        if (currentQuery) {
            setQueryMessages([
                { type: 'user', text: currentQuery.query, timestamp: currentQuery.timestamp },
                {
                    type: 'system',
                    text: `Query completed in ${currentQuery.timeTaken}ms`,
                    timestamp: currentQuery.timestamp + currentQuery.timeTaken,
                    timeTaken: currentQuery.timeTaken
                }
            ]);
        }
    }, [currentQueryIndex, getCurrentQuery]);

    // Check admin status
    useEffect(() => {
        const checkAdminStatus = async () => {
            if (ClanopediaActor && collectionId && principal) {
                const adminStatus = await ClanopediaActor.is_admin_check(collectionId, principal);
                setIsAdmin(adminStatus);
            }
        };
        checkAdminStatus();
    }, [ClanopediaActor, collectionId, principal]);

    // Reset query state when collection changes
    useEffect(() => {
        if (collectionId) {
            // Reset query store for the new collection
            resetQueries();
            setQueryMessages([]);
            setSearchTerm('');
            setCurrentQueryIndex(0);
        }
    }, [collectionId]);

    // Add this effect to fetch metrics
    useEffect(() => {
        const fetchMetrics = async () => {
            if (ClanopediaActor && collectionId) {
                try {
                    const result = await ClanopediaActor.get_collection_metrics_endpoint(collectionId);
                    if ('Ok' in result) {
                        setMetrics(result.Ok);
                    }
                } catch (error) {
                    console.error('Failed to fetch collection metrics:', error);
                    // Set default metrics if fetch fails
                    setMetrics({
                        document_count: BigInt(0),
                        search_count: BigInt(0)
                    });
                }
            }
        };
        fetchMetrics();
    }, [ClanopediaActor, collectionId]);

    const handleQuery = async () => {
        if (searchTerm.trim() === "" || !currentCollection || !BluebandActor) return;

        setIsQuerying(true);
        const startTime = Date.now();

        try {
            const searchRequest = {
                collection_id: currentCollection.id,
                query: searchTerm,
                limit: [10],
                min_score: [0.1],
                filter: [],
                use_approximate: [true]
            };

            const result = await BluebandActor.search(searchRequest);
            
            if ('Ok' in result) {
                // Fetch metadata for each document
                const searchResults = await Promise.all(result.Ok.map(async (match: { 
                    document_id: string; 
                    score: number; 
                    text: string;
                }): Promise<SearchResult> => {
                    // Get document metadata
                    const metadataResult = await BluebandActor.get_document(currentCollection.id, match.document_id);
                    console.log('Document metadata:', metadataResult); // Debug log
                    
                    // Get the first metadata entry since it's returned as an array
                    const metadata = Array.isArray(metadataResult) ? metadataResult[0] : null;
                    
                    // Get the first line as title if not provided
                    const firstLine = match.text.split('\n')[0] || '';
                    const cleanFirstLine = firstLine.replace(/^#+\s*/, '').trim();
                    
                    // Ensure we have a valid title
                    const title = metadata?.title || cleanFirstLine || 'Untitled';
                    console.log('Document title:', title); // Debug log
                    
                    // Ensure we have a valid source URL
                    const sourceUrl = metadata?.source_url?.[0] || null;
                    console.log('Document source URL:', sourceUrl); // Debug log

                    // Convert nanoseconds to milliseconds for the date
                    const timestamp = metadata?.timestamp ? Number(metadata.timestamp) / 1_000_000 : 0;
                    console.log('Timestamp (ms):', timestamp); // Debug log
                    
                    const searchResult: SearchResult = {
                        id: match.document_id,
                        score: Math.round(match.score * 100),
                        title: title,
                        date: new Date(timestamp).toLocaleDateString('en-US', { 
                            month: 'short', 
                            year: 'numeric' 
                        }),
                        author: metadata?.author?.[0] || 'Unknown',
                        sourceLink: sourceUrl,
                        text: match.text
                    };
                    
                    return searchResult;
                }));

                const sortedResults = searchResults.sort((a: SearchResult, b: SearchResult) => b.score - a.score);
                const timeTaken = Date.now() - startTime;
                addQuery(collectionId!, searchTerm, sortedResults, timeTaken);
                setSearchTerm('');
            } else {
                console.error('Search failed with error:', result.Err);
                toast({
                    title: "Search failed",
                    description: `Error: ${JSON.stringify(result.Err)}`,
                    variant: "destructive",
                });
            }
        } catch (error) {
            console.error('Search error:', error);
            toast({
                title: "Search error",
                description: error instanceof Error ? error.message : "An unexpected error occurred",
                variant: "destructive",
            });
        } finally {
            setIsQuerying(false);
        }
    };

    const handlePreviousQuery = () => {
        if (hasPreviousQuery()) {
            setCurrentQueryIndex(currentQueryIndex - 1);
        }
    };

    const handleNextQuery = () => {
        if (hasNextQuery()) {
            setCurrentQueryIndex(currentQueryIndex + 1);
        }
    };

    const handleReadDocument = (docId: string) => {
        setSelectedDocumentId(docId);
        setCurrentView('reader');
    };

    const handleBackToSearch = () => {
        setCurrentView('search');
        setSelectedDocumentId(null);
    };

    if (isLoading) {
        return (
            <div className="min-h-screen bg-background p-8">
                <div className="mx-auto max-w-4xl space-y-8">
                    <Skeleton className="h-12 w-3/4" />
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
                        <Skeleton className="h-[400px]" />
                        <Skeleton className="h-[400px]" />
                    </div>
                </div>
            </div>
        );
    }

    if (!currentCollection) {
        return <div className="text-center py-10">Collection not found.</div>;
    }

    const currentQuery = getCurrentQuery();

    return (
        <div className="min-h-screen bg-background p-8">
            <div className="mx-auto max-w-4xl space-y-8">
                <header className="flex items-center justify-between mb-8">
                    <h1 className="text-3xl font-extrabold tracking-tight text-foreground">
                        <Link to="/" className="text-muted-foreground hover:text-foreground">←</Link> {currentCollection.name}
                        {isAdmin && (
                            <span className="ml-2 text-sm bg-blue-100 text-blue-800 px-2 py-1 rounded">
                                {currentCollection.isOwner ? 'Owner' : 'Admin'}
                            </span>
                        )}
                    </h1>
                    <div className="flex items-center space-x-4">
                        {isAdmin && (
                            <Button variant="ghost" size="icon" asChild>
                                <Link to={`/settings/${collectionId}`}>⚙️</Link>
                            </Button>
                        )}
                        <span className="text-lg font-medium">👥 {currentCollection.admins.length} Admins</span>
                    </div>
                </header>

                <div className="flex items-center justify-between mb-6">
                    <div className="space-y-1">
                        <h1 className="text-2xl font-bold">{currentCollection?.name || 'Collection'}</h1>
                        {currentCollection && (
                            <p className="text-sm text-muted-foreground">
                                {currentCollection.description}
                            </p>
                        )}
                    </div>
                    {isAdmin && currentCollection && (
                        <Button
                            onClick={() => setIsAddDocumentModalOpen(true)}
                            className="bg-primary hover:bg-primary/90"
                        >
                            Add Document
                        </Button>
                    )}
                </div>

                {currentView === 'search' ? (
                    <div className={`grid ${currentQuery ? 'grid-cols-1 md:grid-cols-2' : 'grid-cols-1'} gap-8`}>
                        {/* Left Panel: Query Interface */}
                        <div className={`space-y-4 ${!currentQuery ? 'w-full' : ''}`}>
                            <Card className="h-full flex flex-col">
                                <CardHeader>
                                    <CardTitle>🔍 Document Query</CardTitle>
                                    <p className="text-muted-foreground">Search through collection</p>
                                </CardHeader>
                                <CardContent className="flex-1 overflow-y-auto space-y-4">
                                    {queryMessages.map((msg, index) => (
                                        <div key={index} className={`flex ${msg.type === 'user' ? 'justify-end' : 'justify-start'}`}>
                                            <div className={`p-3 rounded-lg max-w-[80%] ${msg.type === 'user' ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground'}`}>
                                                {msg.text}
                                                {msg.timeTaken && <span className="text-xs opacity-75 ml-2">({msg.timeTaken}ms)</span>}
                                            </div>
                                        </div>
                                    ))}
                                    {isQuerying && (
                                        <div className="flex justify-start">
                                            <div className="p-3 rounded-lg bg-muted text-muted-foreground">
                                                Querying...
                                            </div>
                                        </div>
                                    )}
                                </CardContent>
                                <CardContent className="pt-0 mt-6">
                                    <div className="flex space-x-2">
                                        <Input
                                            placeholder="Type to find something similar..."
                                            value={searchTerm}
                                            onChange={(e) => setSearchTerm(e.target.value)}
                                            onKeyDown={(e) => e.key === 'Enter' && handleQuery()}
                                            disabled={isQuerying}
                                        />
                                        <Button onClick={handleQuery} disabled={isQuerying}>
                                            {isQuerying ? (
                                                <>
                                                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                                    Searching...
                                                </>
                                            ) : (
                                                'Search'
                                            )}
                                        </Button>
                                    </div>
                                </CardContent>
                            </Card>
                        </div>

                        {/* Right Panel: Search Results */}
                        {currentQuery && (
                            <div className="space-y-4">
                                <div className="flex items-center justify-between mb-4">
                                    <div className="flex items-center space-x-2">
                                        <Button
                                            variant="ghost"
                                            size="icon"
                                            onClick={handlePreviousQuery}
                                            disabled={!hasPreviousQuery()}
                                            className="mt-[-2rem]"
                                        >
                                            ←
                                        </Button>
                                        <Button
                                            variant="ghost"
                                            size="icon"
                                            onClick={handleNextQuery}
                                            disabled={!hasNextQuery()}
                                            className="mt-[-2rem]"
                                        >
                                            →
                                        </Button>
                                    </div>
                                    <span className="text-sm text-muted-foreground mt-[-2rem]">
                                        Query {currentQueryIndex + 1} of {useQueryStore.getState().queries.length}
                                    </span>
                                </div>
                                <Card className="h-full flex flex-col">
                                    <CardHeader>
                                        <CardTitle>📄 Search Results ({currentQuery.results.length} found)</CardTitle>
                                    </CardHeader>
                                    <CardContent className="flex-1 overflow-y-auto space-y-6">
                                        <div className="h-[calc(100vh-300px)] overflow-y-auto">
                                            {currentQuery.results.map((result, index) => (
                                                <div key={`${result.id}-${index}`} className="space-y-1 border-b pb-4 last:border-b-0 last:pb-0">
                                                    <p className="text-lg font-semibold">⭐ {result.score}% "{result.title}"</p>
                                                    <p className="text-sm text-muted-foreground">
                                                        📅 {result.date}  👤 {result.author}
                                                    </p>
                                                    <div className="flex space-x-2 mt-2">
                                                        <Button variant="link" onClick={() => handleReadDocument(result.id)} className="p-0 h-auto underline">
                                                            📖 Read
                                                        </Button>
                                                        {result.sourceLink && (
                                                            <Button variant="link" asChild className="p-0 h-auto underline">
                                                                <a 
                                                                    href={result.sourceLink} 
                                                                    target="_blank" 
                                                                    rel="noopener noreferrer"
                                                                    onClick={(e) => e.stopPropagation()}
                                                                >
                                                                    🔗 Source
                                                                </a>
                                                            </Button>
                                                        )}
                                                    </div>
                                                    {result.sourceLink && (
                                                        <p className="text-xs text-muted-foreground mt-1">
                                                            Source: <a 
                                                                href={result.sourceLink} 
                                                                target="_blank" 
                                                                rel="noopener noreferrer"
                                                                className="text-primary hover:underline"
                                                                onClick={(e) => e.stopPropagation()}
                                                            >
                                                                {result.sourceLink}
                                                            </a>
                                                        </p>
                                                    )}
                                                </div>
                                            ))}
                                        </div>
                                    </CardContent>
                                </Card>
                            </div>
                        )}
                    </div>
                ) : (
                    <DocumentReader
                        documentId={selectedDocumentId}
                        onBack={handleBackToSearch}
                        collectionName={currentCollection.name}
                        collectionId={collectionId!}
                        searchQuery={searchTerm}
                    />
                )}

                {/* Collection Stats */}
                {currentView === 'search' && (
                    <Card className="w-full">
                        <CardHeader>
                            <CardTitle>Collection Stats</CardTitle>
                        </CardHeader>
                        <CardContent className="text-sm text-muted-foreground space-y-2">
                            <p>
                                📄 {metrics?.document_count ? Number(metrics.document_count).toLocaleString() : '0'} documents •
                                🔍 {metrics?.search_count ? Number(metrics.search_count).toLocaleString() : '0'} searches •
                                ⚡ {Object.keys(currentCollection.proposals).length} proposals
                            </p>
                            <p>
                                🏛️ {formatGovernanceModel(currentCollection.governance_model)} •
                                👥 {currentCollection.admins.length} admins
                            </p>
                        </CardContent>
                    </Card>
                )}

                <div className="flex justify-around gap-4 mt-8">
                    {isAdmin && (
                        <>
                            <Button variant="outline" asChild>
                                <Link to={`/collections/${collectionId}/proposals`}>🗳️ View Proposals</Link>
                            </Button>
                            <Button variant="outline" asChild>
                                <Link to={`/collections/${collectionId}/settings`}>⚙️ Collection Settings</Link>
                            </Button>
                        </>
                    )}
                </div>

                {/* Add Document Modal - only render if admin */}
                {isAdmin && currentCollection && (
                    <Modal isOpen={isAddDocumentModalOpen} onClose={() => setIsAddDocumentModalOpen(false)}>
                        <DocumentAdder
                            collectionId={currentCollection.id}
                            collectionName={currentCollection.name}
                            onClose={() => setIsAddDocumentModalOpen(false)}
                        />
                    </Modal>
                )}
            </div>
        </div>
    );
} 