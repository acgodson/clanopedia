import GlassContainer from "../lib/components/molecules/glass-container";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "../lib/components/molecules/tabs";
import IntentsFeed from "../lib/components/organisms/docs-feed";
import QueryIntents from "../lib/components/organisms/query-intents";
import LayoutWrapper from "../lib/components/template/layout-wrapper";

const IntentsHome = () => {
  return (
    <LayoutWrapper>
      <GlassContainer>
        <p style={{
            fontFamily: 'monospace'
        }} className="pt-4 px-4 pb-2 font-semibold text-xl">CLANOPEDIA</p>
        <Tabs defaultValue="query">
          <TabsList className="grid w-full grid-cols-2 gap-2">
            <TabsTrigger value="query">Query</TabsTrigger>
            <TabsTrigger value="feed">Feed</TabsTrigger>
          </TabsList>
          <TabsContent className="rounded-2xl bg-[#F8F8F7] p-4" value="query">
            <QueryIntents />
          </TabsContent>
          <TabsContent className="rounded-2xl bg-[#F8F8F7] p-4" value="feed">
            <IntentsFeed />
          </TabsContent>
        </Tabs>
      </GlassContainer>
    </LayoutWrapper>
  );
};

export default IntentsHome;
