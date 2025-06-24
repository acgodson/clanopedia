import { Link } from 'react-router-dom';
import { Button } from '../components/atoms/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/atoms/card';
import { useAuth } from '../providers/useAuth';
import { BookOpen, Users, Shield, ArrowRight } from 'lucide-react';

export function HomePage() {
    const { isAuthenticated } = useAuth();

    return (
        <div className="relative min-h-[calc(100vh-5rem)]">
            {/* Chess pattern background */}
            <div className="absolute inset-0 chess-pattern" />

            {/* Content */}
            <div className="relative">
                {/* Hero Section */}
                <section className="py-20 text-center">
                    <h1 className="text-5xl font-bold mb-6 gradient-text">
                        Welcome to Clanopedia
                    </h1>
                    <p className="text-xl text-muted-foreground max-w-2xl mx-auto mb-8">
                        Your decentralized knowledge base for managing and sharing insights
                    </p>
                    <div className="flex justify-center gap-4">
                        <Button size="lg" className="btn-refined" asChild>
                            <Link to="/collections">
                                Explore Collections <ArrowRight className="ml-2 h-4 w-4" />
                            </Link>
                        </Button>
                        {!isAuthenticated && (
                            <Button size="lg" variant="outline" className="btn-refined">
                                Learn More
                            </Button>
                        )}
                    </div>
                </section>

                {/* Features Section */}
                <section className="py-20">
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-8 max-w-6xl mx-auto px-4">
                        <Card className="glass-card hover-card">
                            <CardHeader>
                                <BookOpen className="h-8 w-8 text-primary mb-4" />
                                <CardTitle className="heading-refined">Knowledge Management</CardTitle>
                                <CardDescription className="text-refined">
                                    Organize and access your clan's knowledge in a structured, searchable format
                                </CardDescription>
                            </CardHeader>
                            <CardContent>
                                <ul className="space-y-2 text-muted-foreground">
                                    <li>• Document organization</li>
                                    <li>• Advanced search</li>
                                    <li>• Version control</li>
                                </ul>
                            </CardContent>
                        </Card>

                        <Card className="glass-card hover-card">
                            <CardHeader>
                                <Users className="h-8 w-8 text-primary mb-4" />
                                <CardTitle className="heading-refined">Collaborative Governance</CardTitle>
                                <CardDescription className="text-refined">
                                    Make decisions together with transparent, on-chain governance
                                </CardDescription>
                            </CardHeader>
                            <CardContent>
                                <ul className="space-y-2 text-muted-foreground">
                                    <li>• Proposal creation</li>
                                    <li>• Voting system</li>
                                    <li>• Permission management</li>
                                </ul>
                            </CardContent>
                        </Card>

                        <Card className="glass-card hover-card">
                            <CardHeader>
                                <Shield className="h-8 w-8 text-primary mb-4" />
                                <CardTitle className="heading-refined">Secure & Decentralized</CardTitle>
                                <CardDescription className="text-refined">
                                    Built on the Internet Computer for maximum security and reliability
                                </CardDescription>
                            </CardHeader>
                            <CardContent>
                                <ul className="space-y-2 text-muted-foreground">
                                    <li>• Blockchain security</li>
                                    <li>• Data integrity</li>
                                    <li>• Access control</li>
                                </ul>
                            </CardContent>
                        </Card>
                    </div>
                </section>

                {/* CTA Section */}
                <section className="py-20 text-center">
                    <div className="max-w-3xl mx-auto px-4">
                        <h2 className="text-3xl font-bold mb-4 gradient-text">
                            Ready to Get Started?
                        </h2>
                        <p className="text-lg text-muted-foreground mb-8">
                            Create your first collection and start organizing your clan's knowledge today
                        </p>
                        <Button size="lg" className="btn-refined" asChild>
                            <Link to="/collections">
                                Create Collection <ArrowRight className="ml-2 h-4 w-4" />
                            </Link>
                        </Button>
                    </div>
                </section>
            </div>
        </div>
    );
} 