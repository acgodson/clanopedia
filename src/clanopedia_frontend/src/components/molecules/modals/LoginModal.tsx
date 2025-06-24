import React from 'react';
import { useAuth } from '../../../providers/useAuth';
import { CardContent, CardDescription, CardHeader, CardTitle } from '../../atoms/card';
import { Button } from '../../atoms/button';
import { useToast } from '../../../providers/toast';

interface LoginModalProps {
  onClose: () => void;
}

export const LoginModal = ({ onClose }: LoginModalProps) => {
  const { isAuthenticated, login, logout, principal, isLoading } = useAuth();
  const { toast } = useToast();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-6">
        <p>Loading...</p>
      </div>
    );
  }

  const handleLogout = async () => {
    await logout();
    toast({
      title: "Logged out",
      description: "You have been successfully logged out",
    });
    onClose(); // Close modal on logout
  };

  const handleLogin = async () => {
    await login();
    toast({
      title: "Login initiated",
      description: "Please complete the login process in the Internet Identity window",
    });
    // The modal might close automatically if the login redirects or on success via useAuth's useEffect
  };

  return (
    <div className="w-full max-w-md">
      <CardHeader>
        <CardTitle>
          {isAuthenticated ? '✅ Authenticated' : '🔐 Login to Clanopedia'}
        </CardTitle>
        <CardDescription>
          {isAuthenticated 
            ? 'You are currently authenticated with Internet Identity'
            : 'Connect your wallet to access full features'}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isAuthenticated ? (
          <div className="mb-4">
            <p className="text-sm text-muted-foreground">
              <span className="font-medium">Principal:</span> {principal?.toString()}
            </p>
          </div>
        ) : (
          <div className="flex flex-col space-y-4">
            <Button onClick={handleLogin} className="flex items-center justify-center">
              <img src="/icp-logo.png" alt="ICP Logo" className="h-5 w-5 mr-2" />
              Internet Identity
            </Button>
            <Button variant="outline" disabled>
              Connect Wallet
            </Button>
          </div>
        )}
      </CardContent>
      <CardContent className="flex justify-end">
        {isAuthenticated && (
          <Button variant="destructive" onClick={handleLogout}>
            Logout
          </Button>
        )}
      </CardContent>
    </div>
  );
}; 