import { useAuth } from '../../providers/useAuth';
import { Button } from '../atoms/button';
import { useToast } from '../../providers/toast';
import { Link } from 'react-router-dom';
import { useTheme } from '../../providers/theme';
import { useState, useRef } from 'react';

interface NavbarProps {
  onLoginClick: () => void;
}

export function Navbar({ onLoginClick }: NavbarProps) {
  const { isAuthenticated, login, logout, principal, isLoading } = useAuth();
  const { toast } = useToast();
  const { theme } = useTheme();
  const [popoverOpen, setPopoverOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const handleCopyPrincipal = async () => {
    if (principal) {
      await navigator.clipboard.writeText(principal.toString());
      toast({
        title: 'Copied',
        description: 'Principal copied to clipboard',
      });
      setPopoverOpen(false);
    }
  };

  const handleLogout = async () => {
    await logout();
    toast({
      title: 'Logged out',
      description: 'You have been successfully logged out',
    });
    setPopoverOpen(false);
  };

  const handleAuth = async () => {
    if (isAuthenticated) {
      setPopoverOpen((open) => !open);
    } else {
      onLoginClick();
    }
  };

  return (
    <nav className="fixed top-0 left-0 right-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container mx-auto px-4">
        <div className="flex h-20 items-center justify-between">
          <Link to="/" className="flex items-center space-x-2">
            <img
              src={theme === 'dark' ? '/CLANOPEDIA.png' : '/CLANOPEDIA-blk.png'}
              alt="Clanopedia"
              className="h-32 w-auto object-contain"
            />
          </Link>
          <div className="flex items-center space-x-4">
            {/* Search Input */}
            <input
              type="text"
              placeholder="🔍 Search Collections"
              className="px-3 py-2 rounded-md border border-input bg-background text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent"
            />

            {!isLoading && (
              <div className="relative">
                <Button
                  ref={buttonRef}
                  variant={isAuthenticated ? "outline" : "default"}
                  onClick={handleAuth}
                >
                  {isAuthenticated
                    ? `Connected: ${principal?.toString().slice(0, 8)}...${principal?.toString().slice(-5)}`
                    : "Login"
                  }
                </Button>
                {isAuthenticated && popoverOpen && (
                  <div className="absolute right-0 mt-2 w-48 bg-popover border border-border rounded-md shadow-lg z-50 animate-fade-in">
                    <button
                      className="w-full text-left px-4 py-2 hover:bg-muted text-sm"
                      onClick={handleCopyPrincipal}
                    >
                      Copy Principal
                    </button>
                    <button
                      className="w-full text-left px-4 py-2 hover:bg-muted text-sm text-red-600"
                      onClick={handleLogout}
                    >
                      Log Out
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </nav>
  );
} 